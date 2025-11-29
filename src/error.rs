#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Setting up error reporting failed")]
    InstallingColorEyre(#[source] color_eyre::Report),

    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),

    #[error("DDP error")]
    Ddp(#[from] ddp_rs::error::DDPError),

    #[error("Failed to bind UDP socket")]
    UDPBind(#[source] std::io::Error),

    #[error("MQTT error")]
    Mqtt(#[source] MqttError),
}

#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("Failed to subscribe")]
    Subscribing(#[source] rumqttc::v5::ClientError),
}
