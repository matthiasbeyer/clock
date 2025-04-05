use embedded_io_async::Write;

pub struct BufWrite<'buf, const SIZE: usize> {
    buf: &'buf mut [u8; SIZE],
    offset: usize,
}

impl<'buf, const SIZE: usize> BufWrite<'buf, SIZE> {
    pub fn new(buf: &'buf mut [u8; SIZE]) -> Self {
        Self { buf, offset: 0 }
    }
}

impl<const SIZE: usize> mqtt_format::v5::write::WriteMqttPacket for BufWrite<'_, SIZE> {
    type Error = mqtt_format::v5::write::MqttWriteError;

    fn write_byte(&mut self, u: u8) -> mqtt_format::v5::write::WResult<Self> {
        self.buf[self.offset] = u;
        self.offset += 1;
        Ok(())
    }

    fn write_slice(&mut self, u: &[u8]) -> mqtt_format::v5::write::WResult<Self> {
        for b in u {
            self.buf[self.offset] = *b;
            self.offset += 1;
        }

        Ok(())
    }
}

pub async fn write_mqtt_packet_to_socket<'a>(
    packet: mqtt_format::v5::packets::MqttPacket<'_>,
    socket: &mut embassy_net::tcp::TcpSocket<'a>,
) -> Result<(), embassy_net::tcp::Error> {
    let mut buf = [0; 1024 * 2];
    packet.write(&mut BufWrite::new(&mut buf));
    socket.write_all(&buf).await
}
