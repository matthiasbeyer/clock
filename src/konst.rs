use embassy_net::StackResources;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::InterruptHandler;
use embedded_graphics::pixelcolor::Rgb888;
use static_cell::StaticCell;

pub(crate) const GREEN: Rgb888 = Rgb888::new(0, 100, 0);

pub(crate) const YELLOW: Rgb888 = Rgb888::new(100, 100, 0);

pub(crate) const RED: Rgb888 = Rgb888::new(100, 0, 0);

pub const NTP_SERVER: &str = env!("NTP_SERVER");

pub(crate) const MQTT_BROKER_ADDR: &str = env!("MQTT_BROKER_ADDR");

pub(crate) const MQTT_BROKER_PORT: u16 = match u16::from_str_radix(env!("MQTT_BROKER_PORT"), 10) {
    Err(_error) => panic!("MQTT_BROKER_PORT is not a valid u16"),
    Ok(port) => port,
};

pub(crate) const MQTT_USER: &str = env!("MQTT_USER");

pub(crate) const MQTT_PASSWORD: &str = env!("MQTT_PASSWORD");

pub(crate) const MQTT_CLIENT_ID: &str = env!("MQTT_CLIENT_ID");

pub(crate) const MQTT_TOPIC_DEVICE_STATE: &str =
    concat!("device/", env!("MQTT_DEVICE_ID"), "/state");

macro_rules! topic {
    (state: $name:literal) => {
        concat!("device/", env!("MQTT_DEVICE_ID"), "/state/", $name)
    };

    (command: $name:literal) => {
        concat!("device/", env!("MQTT_DEVICE_ID"), "/command/", $name)
    };
}

pub(crate) const MQTT_TOPIC_CURRENT_PROGRAM: &str = topic!(state: "current_program");

pub(crate) const MQTT_TOPIC_START_PROGRAM: &str = topic!(command: "start_program");

pub(crate) const MQTT_TOPIC_TIMEZONE_OFFSET: &str = topic!(command: "timezone_offset");

pub(crate) const MQTT_TOPIC_SET_COLOR: &str = topic!(command: "set_color");

pub(crate) const WIFI_NETWORK: &str = env!("WIFI_NETWORK");

pub(crate) const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

pub const NUM_LEDS: usize = 512;

pub const NUM_LEDS_X: usize = 32;

pub const NUM_LEDS_Y: usize = 16;

embassy_rp::bind_interrupts!(pub struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

pub(crate) static NETWORK_STACK_RESOURCES: StaticCell<StackResources<6>> = StaticCell::new();

pub(crate) static NETWORK_STATE: StaticCell<cyw43::State> = StaticCell::new();

pub(crate) static FIRMWARE_FW: &[u8] = include_bytes!(env!("CYW43_FIRMWARE_BIN"));

pub(crate) static FIRMWARE_CLM: &[u8] = include_bytes!(env!("CYW43_FIRMWARE_CLM_BIN"));
