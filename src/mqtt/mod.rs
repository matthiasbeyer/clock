use cloudmqtt_core::client::MqttInstant;
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;

mod error;
mod keep_alive;
mod stack;
mod util;

pub use self::error::MqttClientError;
pub use self::keep_alive::MqttKeepAliver;
pub use self::stack::MqttStackResources;

pub struct MqttClient<'network> {
    fsm: cloudmqtt_core::client::MqttClientFSM,
    socket: TcpSocket<'network>,
}

#[inline]
fn current_time(clock: &crate::clock::Clock) -> MqttInstant {
    MqttInstant::new(clock.get_current_time().as_secs())
}

impl<'network> MqttClient<'network> {
    pub async fn new<'clock>(
        clock: &'clock crate::clock::Clock,
        network_stack: embassy_net::Stack<'network>,
        mqtt_stack_resources: &'network mut MqttStackResources,
        keep_aliver: &MqttKeepAliver,
    ) -> Result<MqttClient<'network>, MqttClientError> {
        let mut socket = TcpSocket::new(
            network_stack,
            &mut mqtt_stack_resources.rx_buffer,
            &mut mqtt_stack_resources.tx_buffer,
        );
        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));
        socket.set_keep_alive(Some(embassy_time::Duration::from_secs(5)));

        let addrs = network_stack
            .dns_query(crate::MQTT_BROKER_ADDR, DnsQueryType::A)
            .await
            .map_err(|error| {
                defmt::error!(
                    "Failed to run DNS query for {}: {:?}",
                    crate::MQTT_BROKER_ADDR,
                    error
                );

                MqttClientError::RunDns(error)
            })?;

        if addrs.is_empty() {
            defmt::error!("Failed to resolve DNS {}", crate::MQTT_BROKER_ADDR);
            return Err(MqttClientError::ResolveDns);
        }

        let mqtt_addr = addrs[0];
        defmt::info!(
            "connecting to MQTT Broker: {}:{}",
            mqtt_addr,
            crate::MQTT_BROKER_PORT
        );

        match socket.connect((mqtt_addr, crate::MQTT_BROKER_PORT)).await {
            Err(error) => {
                defmt::error!(
                    "Failed to connect to MQTT Broker ({}:{}): {:?}",
                    mqtt_addr,
                    crate::MQTT_BROKER_PORT,
                    error
                );
                return Err(MqttClientError::Connect(error));
            }
            Ok(c) => c,
        };
        defmt::info!("Connected to MQTT broker!");

        let mut fsm = cloudmqtt_core::client::MqttClientFSM::new(
            cloudmqtt_core::client::UsizePacketIdentifierStore::default(),
        );

        fsm.handle_connect(
            current_time(clock),
            mqtt_format::v5::packets::connect::MConnect {
                client_identifier: crate::MQTT_CLIENT_ID,
                username: Some(crate::MQTT_USER),
                password: Some(crate::MQTT_PASSWORD.as_bytes()),
                clean_start: true,
                will: Some(mqtt_format::v5::packets::connect::Will {
                    properties: mqtt_format::v5::packets::connect::ConnectWillProperties::new(),
                    topic: crate::MQTT_TOPIC_DEVICE_STATE,
                    payload: b"disconnected",
                    will_qos: mqtt_format::v5::qos::QualityOfService::AtLeastOnce,
                    will_retain: true,
                }),
                properties: mqtt_format::v5::packets::connect::ConnectProperties::new(),
                keep_alive: keep_aliver.keep_alive.as_secs() as u16,
            },
        );

        defmt::info!("Connecting to MQTT as {}", crate::MQTT_USER);

        let Some(next_action) = fsm.run(current_time(clock)) else {
            return Err(MqttClientError::FsmUnexpectedNoAction);
        };

        match next_action {
            cloudmqtt_core::client::ExpectedAction::SendPacket(mqtt_packet) => {
                util::write_mqtt_packet_to_socket(mqtt_packet, &mut socket)
                    .await
                    .unwrap()
            }
            other => {
                defmt::error!("Unexpected FSM action: {:?}", defmt::Debug2Format(&other));
                return Err(MqttClientError::FsmUnexpectedAction);
            }
        }

        {
            const CONNACK_SIZE: usize =
                core::mem::size_of::<mqtt_format::v5::packets::connack::MConnack>();
            let mut buf = [0; CONNACK_SIZE * 2];
            let read_bytes = read_to_buf(&mut socket, &mut buf).await?;

            match mqtt_format::v5::packets::MqttPacket::parse_complete(&buf[0..read_bytes]) {
                Ok(
                    packet @ mqtt_format::v5::packets::MqttPacket::Connack(
                        mqtt_format::v5::packets::connack::MConnack {
                            session_present: _,
                            reason_code,
                            properties: _,
                        },
                    ),
                ) => {
                    if reason_code != mqtt_format::v5::packets::connack::ConnackReasonCode::Success
                    {
                        defmt::error!("Connack not successful");
                        return Err(MqttClientError::ConnackUnsuccessful(reason_code));
                    } else {
                        defmt::info!("MQTT Connection established");
                        let consumer = fsm.consume(packet);
                        if let Some(action) = consumer.run(current_time(clock)) {
                            defmt::error!(
                                "Expected not to receive an action, but got: {:?}",
                                defmt::Debug2Format(&action)
                            );
                            return Err(MqttClientError::FsmUnexpectedAction);
                        }
                    }
                }

                Ok(other) => {
                    defmt::error!(
                        "Expected CONNACK packet after CONNECT, got: {:?}",
                        defmt::Debug2Format(&other)
                    );
                    return Err(MqttClientError::UnexpectedPacket(
                        "CONNACK",
                        other.get_kind(),
                    ));
                }

                Err(error) => {
                    defmt::error!("Failed to parse packet: {:?}", defmt::Debug2Format(&error));
                    return Err(MqttClientError::MqttPacketParsing);
                }
            }
        }

        // here, we are connected

        {
            // send out subscriptions
            let buf = {
                let mut bytes = [0; core::mem::size_of::<
                    mqtt_format::v5::packets::subscribe::Subscription,
                >() * 3];

                for topic_filter in [
                    crate::MQTT_TOPIC_START_PROGRAM,
                    crate::MQTT_TOPIC_TIMEZONE_OFFSET,
                ] {
                    let sub = mqtt_format::v5::packets::subscribe::Subscription {
                    topic_filter,
                    options: mqtt_format::v5::packets::subscribe::SubscriptionOptions {
                        quality_of_service: mqtt_format::v5::qos::QualityOfService::AtMostOnce,
                        no_local: true,
                        retain_as_published: true,
                        retain_handling: mqtt_format::v5::packets::subscribe::RetainHandling::SendRetainedMessagesAlways,
                    }
                };

                    sub.write(&mut util::BufWrite::new(&mut bytes)).unwrap();
                }

                bytes
            };

            fsm.subscribe(
                current_time(clock),
                mqtt_format::v5::packets::subscribe::MSubscribe {
                    packet_identifier: mqtt_format::v5::variable_header::PacketIdentifier(
                        core::num::NonZero::new(1).unwrap(), // will never fail
                    ),
                    properties: mqtt_format::v5::packets::subscribe::SubscribeProperties::new(),
                    subscriptions:
                        mqtt_format::v5::packets::subscribe::Subscriptions::parse_complete(&buf)
                            .expect("Just constructed, should never fail"),
                },
            );
        }

        {
            const SUBACK_SIZE: usize =
                core::mem::size_of::<mqtt_format::v5::packets::suback::MSuback>();
            let mut buf = [0; SUBACK_SIZE * 2];
            let read_bytes = read_to_buf(&mut socket, &mut buf).await?;

            match mqtt_format::v5::packets::MqttPacket::parse_complete(&buf[0..read_bytes]) {
                Ok(
                    packet @ mqtt_format::v5::packets::MqttPacket::Suback(
                        mqtt_format::v5::packets::suback::MSuback { reasons, .. },
                    ),
                ) => {
                    if reasons.iter().any(|reason| {
                        *reason != mqtt_format::v5::packets::suback::SubackReasonCode::GrantedQoS0
                            || *reason
                                != mqtt_format::v5::packets::suback::SubackReasonCode::GrantedQoS1
                            || *reason
                                != mqtt_format::v5::packets::suback::SubackReasonCode::GrantedQoS2
                    }) {
                        return Err(MqttClientError::AnySubscribeFailed);
                    }

                    let consumer = fsm.consume(packet);
                    if let Some(action) = consumer.run(current_time(clock)) {
                        defmt::error!(
                            "Expected not to receive an action, but got: {:?}",
                            defmt::Debug2Format(&action)
                        );
                        return Err(MqttClientError::FsmUnexpectedAction);
                    }
                }

                Ok(other) => {
                    defmt::error!(
                        "Expected CONNACK packet after CONNECT, got: {:?}",
                        defmt::Debug2Format(&other)
                    );
                    return Err(MqttClientError::UnexpectedPacket(
                        "CONNACK",
                        other.get_kind(),
                    ));
                }

                Err(error) => {
                    defmt::error!("Failed to parse packet: {:?}", defmt::Debug2Format(&error));
                    return Err(MqttClientError::MqttPacketParsing);
                }
            }
        }

        defmt::info!("Subscriptions done");

        Ok(Self { fsm, socket })
    }

    pub async fn tick(
        &mut self,
        clock: &crate::clock::Clock,
    ) -> Result<Option<mqtt_format::v5::packets::MqttPacket<'_>>, MqttClientError> {
        match self.fsm.run(current_time(clock)) {
            None => Ok(None),
            Some(cloudmqtt_core::client::ExpectedAction::SendPacket(packet)) => {
                defmt::trace!("Sending out packet: {:?}", packet);
                util::write_mqtt_packet_to_socket(packet, &mut self.socket)
                    .await
                    .map(|_| None)
                    .map_err(MqttClientError::SocketWrite)
            }
            Some(cloudmqtt_core::client::ExpectedAction::SaveClientIdentifier(ident)) => {
                defmt::warn!("Not implemented: Saving client identifier: {}", ident);
                Ok(None)
            }
            Some(cloudmqtt_core::client::ExpectedAction::StorePacket { id }) => {
                defmt::warn!(
                    "Not implemented: Storing packet: {}",
                    defmt::Debug2Format(&id)
                );
                Ok(None)
            }
            Some(cloudmqtt_core::client::ExpectedAction::ReleasePacket { id }) => {
                defmt::warn!(
                    "Not implemented: Releasing packet: {}",
                    defmt::Debug2Format(&id)
                );
                Ok(None)
            }
            Some(cloudmqtt_core::client::ExpectedAction::ReceivePacket(
                cloudmqtt_core::client::ReceivePacket::NoFurtherAction(packet),
            )) => Ok(Some(packet)),
            Some(cloudmqtt_core::client::ExpectedAction::ReceivePacket(
                cloudmqtt_core::client::ReceivePacket::AcknowledgeNeeded {
                    packet,
                    acknowledge,
                },
            )) => match self.fsm.acknowledge(current_time(clock), acknowledge) {
                cloudmqtt_core::client::ExpectedAction::SendPacket(publish) => {
                    util::write_mqtt_packet_to_socket(publish, &mut self.socket)
                        .await
                        .map_err(MqttClientError::SocketWrite)
                        .map(|_| Some(packet))
                }
                other => {
                    defmt::warn!("Expected to receive an ExpectedAction::SendPacket, but got (ignoring): {:?}", defmt::Debug2Format(&other));
                    Ok(Some(packet))
                }
            },
            Some(cloudmqtt_core::client::ExpectedAction::Disconnect) => {
                defmt::warn!("FSM asked us to disconnect");
                Ok(None)
            }
        }
    }

    pub async fn booting(&mut self, clock: &crate::clock::Clock) -> Result<(), MqttClientError> {
        self.publish(clock, crate::MQTT_TOPIC_CURRENT_PROGRAM, "booting").await
    }

    pub async fn current_program(&mut self, clock: &crate::clock::Clock, program_name: &str) -> Result<(), MqttClientError> {
        self.publish(clock, crate::MQTT_TOPIC_CURRENT_PROGRAM, program_name).await
    }

    pub async fn next_payload(&mut self, clock: &crate::clock::Clock) -> Result<Option<MqttPayload>, MqttClientError> {
        Ok(None) // TODO
    }

    async fn publish(&mut self, clock: &crate::clock::Clock, topic_name: &str, text: &str) -> Result<(), MqttClientError> {
        let mut publisher = self
            .fsm
            .publish(mqtt_format::v5::packets::publish::MPublish {
                duplicate: false,
                quality_of_service: mqtt_format::v5::qos::QualityOfService::AtMostOnce,
                retain: false,
                topic_name,
                packet_identifier: None,
                properties: mqtt_format::v5::packets::publish::PublishProperties::new(),
                payload: text.as_bytes(),
            });

        let action = publisher.run(current_time(clock));

        match action {
            Some(cloudmqtt_core::client::ExpectedAction::SendPacket(mqtt_packet)) => {
                if let Err(error) =
                    util::write_mqtt_packet_to_socket(mqtt_packet, &mut self.socket).await
                {
                    defmt::error!("TCP Error {:?}", defmt::Debug2Format(&error));
                    return Err(MqttClientError::TCP);
                } else {
                    Ok(())
                }
            }
            other => {
                defmt::error!("Unexpected FSM action: {:?}", defmt::Debug2Format(&other));
                Err(MqttClientError::FsmUnexpectedAction)
            }
        }
    }
}

async fn read_to_buf(
    tcp_socket: &mut TcpSocket<'_>,
    buf: &mut [u8],
) -> Result<usize, MqttClientError> {
    let mut offset = 0;

    loop {
        match tcp_socket.read(&mut buf[offset..]).await {
            Ok(read) => {
                if read == 0 {
                    // ready with reading
                    defmt::trace!("Finished reading buffer");
                    break;
                }
                defmt::trace!("Read {} bytes into buffer", read);
                offset += read;
            }

            Err(error) => {
                defmt::error!("Failed to read from socket: {:?}", error);
                return Err(MqttClientError::SocketRead(error));
            }
        }
    }

    Ok(offset)
}

pub enum MqttPayload<'p> {
    Timezone(&'p [u8]),
    SetColor(&'p [u8]),
    StartProgram(&'p [u8]),
    Unknown { topic: &'p str, payload: &'p [u8] },
}
