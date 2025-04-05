
#[derive(Debug, thiserror::Error)]
pub enum MqttClientError {
    #[error("Failed to run DNS query for '{}'", crate::MQTT_BROKER_ADDR)]
    RunDns(embassy_net::dns::Error),

    #[error("Failed to resolve DNS for NTP server '{}'", crate::MQTT_BROKER_ADDR)]
    ResolveDns,

    #[error("Internal MQTT statemachine did not return action")]
    FsmUnexpectedNoAction,

    #[error("Internal MQTT statemachine did not return expected action")]
    FsmUnexpectedAction,

    #[error("Failed to connect socket for MQTT client: {:?}", .0)]
    Connect(embassy_net::tcp::ConnectError),

    #[error("Error reading from socket")]
    SocketRead(embassy_net::tcp::Error),

    #[error("Error writing to socket")]
    SocketWrite(embassy_net::tcp::Error),

    #[error("MQTT Connect failed: {:?}", .0)]
    ConnackUnsuccessful(mqtt_format::v5::packets::connack::ConnackReasonCode),

    #[error("Unexpected MQTT packet, expected {}, got {:?}", .0, .1)]
    UnexpectedPacket(&'static str, mqtt_format::v5::packets::MqttPacketKind),

    #[error("Failed to parse MQTT packet")]
    MqttPacketParsing,

    #[error("Any subscribe failed")]
    AnySubscribeFailed,
}

impl defmt::Format for MqttClientError {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "MqttClientError: {}", self)
    }
}
