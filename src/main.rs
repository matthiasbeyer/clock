#![no_std]
#![no_main]

use cyw43::JoinOptions;
use cyw43_pio::PioSpi;
use defmt_rtt as _;
use embassy_executor::Spawner;
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
use static_cell::StaticCell;

mod bounding_box;
mod clock;
mod color;
mod mapping;
mod mqtt;
mod ntp;
mod output;
mod render;
mod util;

pub const NTP_SERVER: &str = env!("NTP_SERVER");

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
    let mut ntp_stack_resources = ntp::NtpStackResources::default();
    let mut mqtt_stack_resources = crate::mqtt::MqttStackResources::default();

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

    let (udp_socket, ntp_client) =
        crate::ntp::NtpClient::new(network_stack, &mut ntp_stack_resources)
            .await
            .unwrap();

    let mut mqtt_client = crate::mqtt::MqttClient::new(network_stack, &mut mqtt_stack_resources)
        .await
        .unwrap();
    defmt::info!("NTP, MQTT setup done!");

    defmt::info!("Starting");

    mqtt_client.booting().await.unwrap();

    let program = PioWs2812Program::new(&mut common);
    let mut leds = PioWs2812::new(&mut common, sm1, p.DMA_CH0, p.PIN_16, &program);

    let result = ntp_client.get_time(&udp_socket).await;
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
            let result = ntp_client.get_time(&udp_socket).await;
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
