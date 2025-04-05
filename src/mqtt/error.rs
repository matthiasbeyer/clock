
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
