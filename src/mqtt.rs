use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;

use crate::konst::MQTT_BROKER_ADDR;
use crate::konst::MQTT_BROKER_PORT;
use crate::konst::MQTT_CLIENT_ID;
use crate::konst::MQTT_TOPIC_CURRENT_PROGRAM;
use crate::konst::MQTT_TOPIC_DEVICE_STATE;
use crate::konst::MQTT_USER;

pub struct MqttClient<'network> {
    client: rust_mqtt::client::client::MqttClient<
        'network,
        TcpSocket<'network>,
        5,
        rust_mqtt::utils::rng_generator::CountingRng,
    >,
}

#[derive(Debug, thiserror::Error)]
pub enum MqttClientError {
    #[error("Failed to run DNS query for '{}'", MQTT_BROKER_ADDR)]
    RunDns(embassy_net::dns::Error),

    #[error("Failed to resolve DNS for NTP server '{}'", MQTT_BROKER_ADDR)]
    ResolveDns,

    #[error("Failed to connect socket for MQTT client: {:?}", .0)]
    Connect(embassy_net::tcp::ConnectError),

    #[error("MQTT Client Network error: {:?}", .0)]
    MqttClient(rust_mqtt::packet::v5::reason_codes::ReasonCode),

    #[error("MQTT Client error: {:?}", .0)]
    MqttError(rust_mqtt::packet::v5::reason_codes::ReasonCode),

    #[error("MQTT Client PING failed: {:?}", .0)]
    Ping(rust_mqtt::packet::v5::reason_codes::ReasonCode),

    #[error("MQTT Client Receive failed: {:?}", .0)]
    Recv(rust_mqtt::packet::v5::reason_codes::ReasonCode),

    #[error("Subscribing to topic '{}' failed: {:?}", .0, .1)]
    Subscribing(
        &'static str,
        rust_mqtt::packet::v5::reason_codes::ReasonCode,
    ),
}

impl defmt::Format for MqttClientError {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "MqttClientError: {}", self)
    }
}

const MQTT_RECV_BUFFER_LEN: usize = 80;
const MQTT_WRITE_BUFFER_LEN: usize = 80;

pub struct MqttStackResources {
    rx_buffer: [u8; 4096],
    tx_buffer: [u8; 4096],

    mqtt_recv_buffer: [u8; MQTT_RECV_BUFFER_LEN],
    mqtt_write_buffer: [u8; MQTT_WRITE_BUFFER_LEN],
}

impl Default for MqttStackResources {
    fn default() -> Self {
        Self {
            rx_buffer: [0; 4096],
            tx_buffer: [0; 4096],

            mqtt_recv_buffer: [0; 80],
            mqtt_write_buffer: [0; 80],
        }
    }
}

impl<'network> MqttClient<'network> {
    pub async fn new(
        network_stack: embassy_net::Stack<'network>,
        mqtt_stack_resources: &'network mut MqttStackResources,
        keep_aliver: &MqttKeepAliver,
    ) -> Result<MqttClient<'network>, MqttClientError> {
        let mut tcp_socket = TcpSocket::new(
            network_stack,
            &mut mqtt_stack_resources.rx_buffer,
            &mut mqtt_stack_resources.tx_buffer,
        );
        tcp_socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));
        tcp_socket.set_keep_alive(Some(embassy_time::Duration::from_secs(5)));

        let addrs = network_stack
            .dns_query(crate::konst::MQTT_BROKER_ADDR, DnsQueryType::A)
            .await
            .map_err(|error| {
                defmt::error!(
                    "Failed to run DNS query for {}: {:?}",
                    MQTT_BROKER_ADDR,
                    error
                );

                MqttClientError::RunDns(error)
            })?;

        if addrs.is_empty() {
            defmt::error!("Failed to resolve DNS {}", MQTT_BROKER_ADDR);
            return Err(MqttClientError::ResolveDns);
        }

        let mqtt_addr = addrs[0];
        defmt::info!(
            "connecting to MQTT Broker: {}:{}",
            mqtt_addr,
            MQTT_BROKER_PORT
        );

        if let Err(error) = tcp_socket
            .connect((mqtt_addr, crate::konst::MQTT_BROKER_PORT))
            .await
        {
            defmt::error!(
                "Failed to connect to MQTT Broker ({}:{}): {:?}",
                mqtt_addr,
                MQTT_BROKER_PORT,
                error
            );
            return Err(MqttClientError::Connect(error));
        }
        defmt::info!("Connected to MQTT broker!");

        let mut config = rust_mqtt::client::client_config::ClientConfig::new(
            rust_mqtt::client::client_config::MqttVersion::MQTTv5,
            rust_mqtt::utils::rng_generator::CountingRng(20000),
        );
        config.add_max_subscribe_qos(rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0);
        config.add_client_id(crate::konst::MQTT_CLIENT_ID);
        config.max_packet_size = 100;
        config.keep_alive = keep_aliver.as_secs();

        defmt::info!("Connecting to MQTT as {}", MQTT_USER);
        // config.add_username(MQTT_USER);
        // config.add_password(MQTT_PASSWORD);

        defmt::info!("Installing WILL message on {}", MQTT_TOPIC_DEVICE_STATE);
        config.add_will(
            crate::konst::MQTT_TOPIC_DEVICE_STATE,
            "disconnected".as_bytes(),
            false,
        );

        let mut client = rust_mqtt::client::client::MqttClient::<_, 5, _>::new(
            tcp_socket,
            &mut mqtt_stack_resources.mqtt_write_buffer,
            MQTT_RECV_BUFFER_LEN,
            &mut mqtt_stack_resources.mqtt_recv_buffer,
            MQTT_WRITE_BUFFER_LEN,
            config,
        );

        match client.connect_to_broker().await {
            Err(rust_mqtt::packet::v5::reason_codes::ReasonCode::NetworkError) => {
                defmt::error!("MQTT Network Error");
                return Err(MqttClientError::MqttClient(
                    rust_mqtt::packet::v5::reason_codes::ReasonCode::NetworkError,
                ));
            }
            Err(error) => {
                defmt::error!("Other MQTT Error: {:?}", error);
                return Err(MqttClientError::MqttError(error));
            }
            Ok(_) => {
                defmt::info!("Connected to broker");
            }
        }

        for topic in [
            crate::konst::MQTT_TOPIC_START_PROGRAM,
            crate::konst::MQTT_TOPIC_TIMEZONE_OFFSET,
        ] {
            match client.subscribe_to_topic(topic).await {
                Ok(()) => defmt::info!("Subscribing to '{}' succeeded", topic),
                Err(error) => {
                    defmt::error!("Subscribing to '{}' failed: {}", topic, error);
                    return Err(MqttClientError::Subscribing(topic, error));
                }
            }
        }
        defmt::info!("Subscriptons done");

        Ok(Self { client })
    }

    pub async fn ping(&mut self) -> Result<(), MqttClientError> {
        self.client.send_ping().await.map_err(MqttClientError::Ping)
    }

    pub async fn booting(&mut self) -> Result<(), MqttClientError> {
        match self
            .client
            .send_message(
                crate::konst::MQTT_TOPIC_DEVICE_STATE,
                "booting".as_bytes(),
                rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0,
                false, // do not retain
            )
            .await
        {
            Err(rust_mqtt::packet::v5::reason_codes::ReasonCode::NetworkError) => {
                defmt::error!("MQTT Network Error");
                Err(MqttClientError::MqttClient(
                    rust_mqtt::packet::v5::reason_codes::ReasonCode::NetworkError,
                ))
            }
            Err(error) => {
                defmt::error!("Other MQTT Error: {:?}", error);
                Err(MqttClientError::MqttError(error))
            }
            Ok(_) => {
                defmt::info!("'booting' message sent successfully");
                Ok(())
            }
        }
    }

    pub async fn current_program(&mut self, program_name: &str) -> Result<(), MqttClientError> {
        match self
            .client
            .send_message(
                crate::konst::MQTT_TOPIC_CURRENT_PROGRAM,
                program_name.as_bytes(),
                rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0,
                true, // do retain
            )
            .await
        {
            Err(rust_mqtt::packet::v5::reason_codes::ReasonCode::NetworkError) => {
                defmt::error!("MQTT Network Error");
                Err(MqttClientError::MqttClient(
                    rust_mqtt::packet::v5::reason_codes::ReasonCode::NetworkError,
                ))
            }
            Err(error) => {
                defmt::error!("Other MQTT Error: {:?}", error);
                Err(MqttClientError::MqttError(error))
            }
            Ok(_) => {
                defmt::info!("'current_program' message sent successfully");
                Ok(())
            }
        }
    }

    pub async fn next_payload(&mut self) -> Result<MqttPayload, MqttClientError> {
        match self
            .client
            .receive_message()
            .await
            .map_err(MqttClientError::Recv)?
        {
            (crate::konst::MQTT_TOPIC_TIMEZONE_OFFSET, payload) => {
                Ok(MqttPayload::Timezone(payload))
            }
            (crate::konst::MQTT_TOPIC_START_PROGRAM, payload) => {
                Ok(MqttPayload::StartProgram(payload))
            }
            (crate::konst::MQTT_TOPIC_SET_COLOR, payload) => Ok(MqttPayload::SetColor(payload)),
            (topic, payload) => Ok(MqttPayload::Unknown { topic, payload }),
        }
    }
}

pub enum MqttPayload<'p> {
    Timezone(&'p [u8]),
    SetColor(&'p [u8]),
    StartProgram(&'p [u8]),
    Unknown { topic: &'p str, payload: &'p [u8] },
}

pub struct MqttKeepAliver {
    last_keep_alive: embassy_time::Instant,
    keep_alive: embassy_time::Duration,
}

impl MqttKeepAliver {
    pub fn new(keep_alive: embassy_time::Duration) -> Self {
        Self {
            last_keep_alive: embassy_time::Instant::now(),
            keep_alive,
        }
    }

    pub fn as_secs(&self) -> u16 {
        self.keep_alive.as_secs() as u16
    }

    pub fn update_to_now(&mut self) {
        self.last_keep_alive = embassy_time::Instant::now();
    }
}

impl crate::render::Renderable for MqttKeepAliver {
    fn get_next_cycle_time(&self) -> embassy_time::Instant {
        self.last_keep_alive
            .checked_add(self.keep_alive / 2)
            .unwrap_or_else(embassy_time::Instant::now)
    }

    fn needs_cycle(&self) -> bool {
        self.last_keep_alive.elapsed() > (self.keep_alive / 2)
    }
}
