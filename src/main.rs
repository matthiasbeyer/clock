#![no_std]
#![no_main]

use core::net::IpAddr;
use core::net::SocketAddr;

use cyw43::JoinOptions;
use cyw43_pio::PioSpi;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;
use embassy_net::udp::PacketMetadata;
use embassy_net::udp::UdpSocket;
use embassy_net::StackResources;
use embassy_rp::bind_interrupts;
use embassy_rp::config::Config;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::DMA_CH1;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::InterruptHandler;
use embassy_rp::pio::Pio;
use embassy_rp::pio_programs::ws2812::PioWs2812;
use embassy_rp::pio_programs::ws2812::PioWs2812Program;
use embassy_time::Duration;
use embassy_time::Timer;
use panic_probe as _;
use render::RenderToDisplay;
use render::Renderable;
use sntpc::NtpContext;
use static_cell::StaticCell;

mod bounding_box;
mod clock;
mod color;
mod mapping;
mod ntp;
mod output;
mod render;
mod util;

const NTP_SERVER: &str = env!("NTP_SERVER");

const MQTT_BROKER_ADDR: &str = env!("MQTT_BROKER_ADDR");
const MQTT_BROKER_PORT: u16 = match u16::from_str_radix(env!("MQTT_BROKER_PORT"), 10) {
    Err(_error) => panic!("MQTT_BROKER_PORT is not a valid u16"),
    Ok(port) => port,
};

const MQTT_USER: &str = env!("MQTT_USER");
const MQTT_PASSWORD: &str = env!("MQTT_PASSWORD");
const MQTT_CLIENT_ID: &str = env!("MQTT_CLIENT_ID");
const MQTT_TOPIC_DEVICE_STATE: &str = concat!("device/", env!("MQTT_DEVICE_ID"), "/state");

const WIFI_NETWORK: &str = env!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

pub const NUM_LEDS: usize = 512;
pub const NUM_LEDS_X: usize = 32;
pub const NUM_LEDS_Y: usize = 16;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

static NETWORK_STACK_RESOURCES: StaticCell<StackResources<6>> = StaticCell::new();

static NETWORK_STATE: StaticCell<cyw43::State> = StaticCell::new();

static FIRMWARE_FW: &[u8] = include_bytes!(env!("CYW43_FIRMWARE_BIN"));
static FIRMWARE_CLM: &[u8] = include_bytes!(env!("CYW43_FIRMWARE_CLM_BIN"));

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH1>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let config = Config::default();
    let p = embassy_rp::init(config);

    let Pio {
        mut common,
        sm0,
        sm1,
        irq0,
        ..
    } = Pio::new(p.PIO0, Irqs);

    let pwr = Output::new(p.PIN_23, embassy_rp::gpio::Level::Low);
    let cs = Output::new(p.PIN_25, embassy_rp::gpio::Level::High);
    let spi = PioSpi::new(
        &mut common,
        sm0,
        cyw43_pio::DEFAULT_CLOCK_DIVIDER,
        irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH1,
    );

    let state = NETWORK_STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, FIRMWARE_FW).await;

    spawner.spawn(cyw43_task(runner)).unwrap();

    control.init(FIRMWARE_CLM).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Configure network stack
    let config = embassy_net::Config::dhcpv4(Default::default());

    // Init network stack
    let (network_stack, runner) = embassy_net::new(
        net_device,
        config,
        NETWORK_STACK_RESOURCES.init(StackResources::new()),
        0,
    );

    // Launch network task
    spawner.spawn(net_task(runner)).unwrap();

    loop {
        match control
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
            .await
        {
            Ok(_) => break,
            Err(err) => {
                defmt::info!("join failed with status={}", err.status);
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    defmt::info!("waiting for DHCP...");
    while !network_stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    defmt::info!("DHCP is now up!");

    defmt::info!("waiting for link up...");
    while !network_stack.is_link_up() {
        Timer::after_millis(500).await;
    }
    defmt::info!("Link is up!");

    // Wait for the tap interface to be up before continuing
    defmt::info!("waiting for stack to be up...");
    network_stack.wait_config_up().await;
    defmt::info!("Stack is up!");

    // Create UDP socket
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    let mut udp_socket = UdpSocket::new(
        network_stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    udp_socket.bind(123).unwrap();

    let context = NtpContext::new(crate::ntp::Timestamp::default());

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    let mut tcp_socket = TcpSocket::new(network_stack, &mut rx_buffer, &mut tx_buffer);
    tcp_socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

    let ntp_addrs = async {
        let addrs = network_stack
            .dns_query(NTP_SERVER, DnsQueryType::A)
            .await
            .map_err(|error| {
                defmt::error!("Failed to run DNS query for {}: {:?}", NTP_SERVER, error);
            })?;

        if addrs.is_empty() {
            defmt::error!("Failed to resolve DNS {}", NTP_SERVER);
            return Err(());
        }

        Ok(addrs)
    };

    let mqtt_addrs = async {
        let addrs = network_stack
            .dns_query(MQTT_BROKER_ADDR, DnsQueryType::A)
            .await
            .map_err(|error| {
                defmt::error!(
                    "Failed to run DNS query for {}: {:?}",
                    MQTT_BROKER_ADDR,
                    error
                );
            })?;

        if addrs.is_empty() {
            defmt::error!("Failed to resolve DNS {}", MQTT_BROKER_ADDR);
            return Err(());
        }
        Ok(addrs)
    };

    let (ntp_addrs, mqtt_addrs) = match embassy_futures::join::join(ntp_addrs, mqtt_addrs).await {
        (Err(()), _) => {
            defmt::error!("Failed to resolve NTP addresses");
            return;
        }
        (_, Err(())) => {
            defmt::error!("Failed to resolve MQTT addresses");
            return;
        }
        (Ok(ntp), Ok(mqtt)) => (ntp, mqtt),
    };

    let _connection = {
        let mqtt_addr = mqtt_addrs[0];
        defmt::info!(
            "connecting to MQTT Broker: {}:{}",
            mqtt_addr,
            MQTT_BROKER_PORT
        );
        match tcp_socket.connect((mqtt_addr, MQTT_BROKER_PORT)).await {
            Err(e) => {
                defmt::error!("Failed to connect to MQTT Broker: {:?}", e);
                return;
            }
            Ok(conn) => {
                defmt::info!("Connected to MQTT broker!");
                conn
            }
        }
    };

    let mut mqtt_recv_buffer = [0; 80];
    let mut mqtt_write_buffer = [0; 80];
    let mut mqtt_client = {
        let mut config = rust_mqtt::client::client_config::ClientConfig::new(
            rust_mqtt::client::client_config::MqttVersion::MQTTv5,
            rust_mqtt::utils::rng_generator::CountingRng(20000),
        );
        config.add_max_subscribe_qos(rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1);
        config.add_client_id(MQTT_CLIENT_ID);
        config.max_packet_size = 100;

        defmt::info!("Connecting to MQTT as {}", MQTT_USER);
        // config.add_username(MQTT_USER);
        // config.add_password(MQTT_PASSWORD);

        defmt::info!("Installing WILL message on {}", MQTT_TOPIC_DEVICE_STATE);
        config.add_will(MQTT_TOPIC_DEVICE_STATE, "disconnected".as_bytes(), false);

        let mut client = rust_mqtt::client::client::MqttClient::<_, 5, _>::new(
            tcp_socket,
            &mut mqtt_write_buffer,
            80,
            &mut mqtt_recv_buffer,
            80,
            config,
        );

        match client.connect_to_broker().await {
            Ok(()) => {}
            Err(mqtt_error) => match mqtt_error {
                rust_mqtt::packet::v5::reason_codes::ReasonCode::NetworkError => {
                    defmt::error!("MQTT Network Error");
                    return;
                }
                _ => {
                    defmt::error!("Other MQTT Error: {:?}", mqtt_error);
                    return;
                }
            },
        }

        client
    };

    defmt::info!("Starting");

    if let Err(mqtt_error) = mqtt_client
        .send_message(
            MQTT_TOPIC_DEVICE_STATE,
            "booting".as_bytes(),
            rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1,
            false, // do not retain
        )
        .await
    {
        match mqtt_error {
            rust_mqtt::packet::v5::reason_codes::ReasonCode::NetworkError => {
                defmt::error!("MQTT Network Error");
                return;
            }
            _ => {
                defmt::error!("Other MQTT Error: {:?}", mqtt_error);
                return;
            }
        }
    }

    let program = PioWs2812Program::new(&mut common);
    let mut leds = PioWs2812::new(&mut common, sm1, p.DMA_CH0, p.PIN_16, &program);

    let addr: IpAddr = ntp_addrs[0].into();

    let result = sntpc::get_time(SocketAddr::from((addr, 123)), &udp_socket, context).await;
    let ntp_result = match result {
        Ok(time) => {
            defmt::info!("Time: {:?}", time);
            time
        }
        Err(e) => {
            defmt::error!("Error getting time: {:?}", e);
            loop {
                embassy_time::Timer::after(Duration::from_secs(60)).await
            }
        }
    };
    let last_clock_update = embassy_time::Instant::now();

    let mut color_iter = crate::color::ColorIter::new(10, embassy_time::Duration::from_secs(1));

    let mut color = color_iter.next().unwrap();
    let mut clock = crate::clock::Clock::new(ntp_result, last_clock_update);
    let mut border = crate::bounding_box::BoundingBox::new();

    loop {
        let cycle_start_time = embassy_time::Instant::now();
        if cycle_start_time.duration_since(last_clock_update) > Duration::from_secs(60) {
            defmt::info!("Updating time");
            let result = sntpc::get_time(SocketAddr::from((addr, 123)), &udp_socket, context).await;
            match result {
                Ok(time) => {
                    defmt::info!("Time: {:?}", time);
                    clock.set_system_time(time, embassy_time::Instant::now());
                }
                Err(e) => {
                    defmt::error!("Error getting time: {:?}", e);
                }
            }
        }

        if color_iter.needs_cycle() {
            color = color_iter.next().unwrap();
        }

        defmt::debug!("Rendering");
        let mut display = output::OutputBuffer::new();
        border.render_to_display(&mut display, color);
        clock.render_to_display(&mut display, color);
        display.render_into(&mut leds).await;
        defmt::debug!("Rendering done");

        let min_cycle_duration = [
            color_iter.get_next_cycle_time(),
            clock.get_next_cycle_time(),
        ]
        .into_iter()
        .min()
        .unwrap_or_else(embassy_time::Instant::now);

        let cycle_duration = embassy_time::Instant::now().duration_since(cycle_start_time);

        if let Some(sleep_until) = min_cycle_duration.checked_sub(cycle_duration) {
            if let Some(sleep_time) =
                sleep_until.checked_duration_since(embassy_time::Instant::now())
            {
                embassy_time::Timer::after(sleep_time).await
            }
        }
    }
}
