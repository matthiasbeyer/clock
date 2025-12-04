#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub display: DisplayConfig,
    pub mqtt: MqttConfig,
}

#[derive(Debug, serde::Deserialize)]
pub struct DisplayConfig {
    pub host: std::net::IpAddr,
    pub port: u16,
    pub udp_port: u16,
    pub initial_brightness: u8,
    #[serde(with = "humantime_serde")]
    pub interval: std::time::Duration,
    pub time_font: Font,
    pub time_offset_x: u8,
    pub time_offset_y: u8,

    /// How long to debounce "TurnOn" events
    #[serde(with = "humantime_serde")]
    pub debounce_turn_on: std::time::Duration,

    pub bootstate: Bootstate,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Bootstate {
    On,
    Off,
}

impl Bootstate {
    pub fn into_bool(self) -> bool {
        std::matches!(self, Bootstate::On)
    }
}

#[derive(Debug, Copy, Clone, serde::Deserialize)]
pub enum Font {
    Font5x8,
    Font6x9,
    Font6x10,
}

impl From<Font> for embedded_graphics::mono_font::MonoFont<'static> {
    fn from(value: Font) -> Self {
        match value {
            Font::Font5x8 => embedded_graphics::mono_font::ascii::FONT_5X8,
            Font::Font6x9 => embedded_graphics::mono_font::ascii::FONT_6X9,
            Font::Font6x10 => embedded_graphics::mono_font::ascii::FONT_6X10,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MqttConfig {
    pub host: std::net::IpAddr,
    pub port: u16,
    pub qos: Qos,
    pub client_name: String,

    pub username: Option<String>,
    pub password: Option<String>,

    #[serde(with = "humantime_serde")]
    pub keep_alive: std::time::Duration,

    pub topic_prefix: String,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[expect(clippy::enum_variant_names, reason = "That's the names")]
pub enum Qos {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl From<Qos> for rumqttc::v5::mqttbytes::QoS {
    fn from(value: Qos) -> Self {
        match value {
            Qos::AtMostOnce => rumqttc::v5::mqttbytes::QoS::AtMostOnce,
            Qos::AtLeastOnce => rumqttc::v5::mqttbytes::QoS::AtLeastOnce,
            Qos::ExactlyOnce => rumqttc::v5::mqttbytes::QoS::ExactlyOnce,
        }
    }
}

impl Config {
    pub async fn load(path: &camino::Utf8Path) -> Result<Self, ConfigError> {
        let config_str =
            tokio::fs::read_to_string(path)
                .await
                .map_err(|source| ConfigError::ReadingFile {
                    path: path.to_path_buf(),
                    source,
                })?;

        toml::from_str(&config_str).map_err(ConfigError::ParsingConfig)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read configuration file from path '{}'", .path)]
    ReadingFile {
        path: camino::Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    ParsingConfig(#[from] toml::de::Error),
}
