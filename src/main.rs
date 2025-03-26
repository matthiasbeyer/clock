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
use embassy_time::WithTimeout;
use embedded_graphics::pixelcolor::Rgb888;
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
mod program;
mod render;
mod text;
mod util;

const GREEN: Rgb888 = Rgb888::new(0, 100, 0);
const YELLOW: Rgb888 = Rgb888::new(100, 100, 0);
const RED: Rgb888 = Rgb888::new(100, 0, 0);

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

macro_rules! topic {
    (state: $name:literal) => {
        concat!("device/", env!("MQTT_DEVICE_ID"), "/state/", $name)
    };

    (command: $name:literal) => {
        concat!("device/", env!("MQTT_DEVICE_ID"), "/command/", $name)
    };
}

const MQTT_TOPIC_CURRENT_PROGRAM: &str = topic!(state: "current_program");
const MQTT_TOPIC_START_PROGRAM: &str = topic!(command: "start_program");
const MQTT_TOPIC_TIMEZONE_OFFSET: &str = topic!(command: "timezone_offset");
const MQTT_TOPIC_SET_COLOR: &str = topic!(command: "set_color");

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

    let program = PioWs2812Program::new(&mut common);
    let mut leds = PioWs2812::new(&mut common, sm1, p.DMA_CH0, p.PIN_16, &program);
    crate::text::render_text_to_leds("Booting", GREEN, &mut leds).await;

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

    crate::text::render_text_to_leds(concat!("WIFI: ", env!("WIFI_NETWORK")), GREEN, &mut leds)
        .await;
    loop {
        match control
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
            .await
        {
            Ok(_) => break,
            Err(err) => {
                crate::text::render_text_to_leds("Wifi failed", RED, &mut leds).await;
                defmt::info!("join failed with status={}", err.status);
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    defmt::info!("waiting for DHCP...");
    crate::text::render_text_to_leds("NET", GREEN, &mut leds).await;
    {
        let mut tries = 0u32;
        while !network_stack.is_config_up() {
            Timer::after_millis(100).await;
            tries += 1;
            match tries {
                0..10 => crate::text::render_text_to_leds("NET.", GREEN, &mut leds).await,
                10..20 => crate::text::render_text_to_leds("NET..", YELLOW, &mut leds).await,
                20.. => crate::text::render_text_to_leds("NET...", YELLOW, &mut leds).await,
            };
        }
    }
    defmt::info!("DHCP is now up!");

    defmt::info!("waiting for link up...");
    crate::text::render_text_to_leds("DHCP", GREEN, &mut leds).await;
    {
        let mut tries = 0u32;
        while !network_stack.is_link_up() {
            Timer::after_millis(500).await;
            tries += 1;
            match tries {
                0..10 => crate::text::render_text_to_leds("DHCP.", GREEN, &mut leds).await,
                10..20 => crate::text::render_text_to_leds("DHCP..", YELLOW, &mut leds).await,
                20.. => crate::text::render_text_to_leds("DHCP...", YELLOW, &mut leds).await,
            };
        }
    }
    defmt::info!("Link is up!");

    // Wait for the tap interface to be up before continuing
    defmt::info!("waiting for stack to be up...");
    network_stack.wait_config_up().await;
    defmt::info!("Stack is up!");

    crate::text::render_text_to_leds("NTP", GREEN, &mut leds).await;
    let Ok((udp_socket, ntp_client)) =
        crate::ntp::NtpClient::new(network_stack, &mut ntp_stack_resources).await
    else {
        crate::text::render_text_to_leds("NTP", RED, &mut leds).await;
        loop {
            Timer::after_secs(1).await;
        }
    };

    let keep_aliver = crate::mqtt::MqttKeepAliver::new(Duration::from_secs(15));

    let Ok(mut mqtt_client) =
        crate::mqtt::MqttClient::new(network_stack, &mut mqtt_stack_resources, &keep_aliver).await
    else {
        crate::text::render_text_to_leds("MQTT", RED, &mut leds).await;
        loop {
            Timer::after_secs(1).await;
        }
    };
    defmt::info!("NTP, MQTT setup done!");

    defmt::info!("Starting");

    mqtt_client.booting().await.unwrap();

    let result = ntp_client.get_time(&udp_socket).await;
    let ntp_result = match result {
        Ok(time) => {
            defmt::info!("Time: {:?}", time);
            time
        }
        Err(e) => {
            crate::text::render_text_to_leds("NTP failed", RED, &mut leds).await;
            defmt::error!("Error getting time: {:?}", e);
            loop {
                embassy_time::Timer::after(Duration::from_secs(60)).await
            }
        }
    };
    let mut last_clock_update = embassy_time::Instant::now();
    let mut last_mqtt_update = embassy_time::Instant::now();

    let color_iter = crate::color::ColorIter::new(10, embassy_time::Duration::from_secs(1));
    let mut color_provider = crate::color::ColorProvider::new(color_iter);

    let mut color = color_provider.next().unwrap();
    let mut clock = crate::clock::Clock::new(ntp_result, last_clock_update);
    let mut border = crate::bounding_box::BoundingBox::new();
    let _current_program: Option<program::ProgramId> = None;

    let _ = mqtt_client.current_program("clock").await;
    let mut keep_aliver = keep_aliver;

    crate::text::render_text_to_leds("Booted", GREEN, &mut leds).await;
    Timer::after_secs(1).await;

    loop {
        let cycle_start_time = embassy_time::Instant::now();

        defmt::debug!("Last mqtt update fetched: {}", last_mqtt_update);
        if cycle_start_time.duration_since(last_mqtt_update) > Duration::from_secs(1) {
            defmt::debug!("Fetching MQTT updates");
            match mqtt_client
                .next_payload()
                .with_timeout(embassy_time::Duration::from_secs(1))
                .await
            {
                Err(_timeout) => {
                    defmt::debug!("Ignoring MQTT recv timeout");
                }
                Ok(Err(mqtt_error)) => defmt::error!("MQTT Error: {:?}", mqtt_error),
                Ok(Ok(payload)) => {
                    handle_next_mqtt_payload(payload, &mut clock, &mut color_provider);
                }
            }

            last_mqtt_update = embassy_time::Instant::now();
            keep_aliver.update_to_now();
        }

        if keep_aliver.needs_cycle() {
            if let Err(error) = mqtt_client.ping().await {
                defmt::error!("Failed to PING: {:?}", defmt::Debug2Format(&error));
            }
            keep_aliver.update_to_now();
        }

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
            last_clock_update = embassy_time::Instant::now();
        }

        if color_provider.needs_cycle() {
            color = color_provider.next().unwrap();
        }

        defmt::debug!("Rendering");
        let mut display = output::OutputBuffer::new();
        border.render_to_display(&mut display, color);
        clock.render_to_display(&mut display, color);
        display.render_into(&mut leds).await;
        defmt::debug!("Rendering done");

        let min_cycle_duration = [
            color_provider.get_next_cycle_time(),
            clock.get_next_cycle_time(),
            keep_aliver.get_next_cycle_time(),
        ]
        .into_iter()
        .min()
        .unwrap_or_else(|| {
            defmt::debug!("Using now()!");
            embassy_time::Instant::now()
        });

        let cycle_duration = embassy_time::Instant::now().duration_since(cycle_start_time);

        defmt::debug!("cycle duration = {}", cycle_duration);
        if let Some(sleep_until) = min_cycle_duration.checked_sub(cycle_duration) {
            defmt::debug!("sleep until = {}", sleep_until);
            if let Some(sleep_time) =
                sleep_until.checked_duration_since(embassy_time::Instant::now())
            {
                defmt::debug!("Sleeping for {}", sleep_time);
                embassy_time::Timer::after(sleep_time).await
            }
        }
    }
}

fn handle_next_mqtt_payload(
    payload: mqtt::MqttPayload,
    clock: &mut crate::clock::Clock,
    color_provider: &mut color::ColorProvider,
) {
    defmt::info!("Handling MQTT payload");
    match payload {
        mqtt::MqttPayload::Timezone(pl) => {
            let s = match core::str::from_utf8(pl) {
                Ok(s) => s,
                Err(_) => {
                    defmt::warn!("{} is not valid UTF8", pl);
                    return;
                }
            };

            match s.parse::<u64>() {
                Ok(secs) => clock.set_timezone_offset(Duration::from_secs(secs)),
                Err(_) => defmt::warn!("Failed to parse {} as u64", pl),
            };
        }

        mqtt::MqttPayload::StartProgram(pl) => {
            todo!()
        }

        mqtt::MqttPayload::SetColor(pl) => {
            #[derive(serde::Deserialize)]
            struct SetColorPayload {
                color: [u8; 3],
                duration_secs: u64,
            }

            match serde_json_core::from_slice::<SetColorPayload>(pl) {
                Ok((pl, n_bytes)) => {
                    defmt::debug!("Deserialized SetColorPayload, consumed {} bytes", n_bytes);
                    color_provider.set_color_for(
                        pl.color,
                        embassy_time::Duration::from_secs(pl.duration_secs),
                    );
                }
                Err(error) => {
                    defmt::warn!(
                        "Failed to deserialize SetColorPayload: {:?}",
                        defmt::Debug2Format(&error)
                    );
                }
            }
        }

        mqtt::MqttPayload::Unknown { topic, payload } => {
            defmt::debug!(
                "Received MQTT message on unhandled topic '{}': {}",
                topic,
                payload
            );
        }
    }
}
