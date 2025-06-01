const MQTT_RECV_BUFFER_LEN: usize = 80;
const MQTT_WRITE_BUFFER_LEN: usize = 80;

pub struct MqttStackResources {
    pub rx_buffer: [u8; 4096],
    pub tx_buffer: [u8; 4096],

    #[allow(unused)] // TODO
    pub mqtt_recv_buffer: [u8; MQTT_RECV_BUFFER_LEN],
    #[allow(unused)] // TODO
    pub mqtt_write_buffer: [u8; MQTT_WRITE_BUFFER_LEN],
}

impl Default for MqttStackResources {
    fn default() -> Self {
        Self {
            rx_buffer: [0; 4096],
            tx_buffer: [0; 4096],

            mqtt_recv_buffer: [0; 80],
            mqtt_write_buffer: [0; 80],
        }
    }
}

