#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Setting up error reporting failed")]
    InstallingColorEyre(#[source] color_eyre::Report),

    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),

    #[error("DDP error")]
    Ddp(#[from] ddp_rs::error::DDPError),

    #[error("Error getting local time")]
    TimeOffset(#[source] time::error::IndeterminateOffset),

    #[error("Error formatting time")]
    TimeFormatting(#[source] time::error::Format),

    #[error("Failed to bind UDP socket")]
    UDPBind(#[source] std::io::Error),

    #[error("MQTT error")]
    Mqtt(#[source] MqttError),

    #[error("URL error")]
    Url(#[from] url::ParseError),

    #[error("Reqwest error")]
    Reqwest(#[source] reqwest::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("Failed to subscribe")]
    Subscribing(#[source] rumqttc::v5::ClientError),

    #[error("Connection failed")]
    Connection(#[source] rumqttc::v5::ConnectionError),
}
