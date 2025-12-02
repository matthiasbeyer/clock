use ddp_rs::connection::DDPConnection;
use rgb::RGB8;

pub struct Writer {
    connection: DDPConnection,
}

impl Writer {
    pub fn new(connection: DDPConnection) -> Self {
        Self { connection }
    }
}

impl smart_leds_trait::SmartLedsWrite for Writer {
    type Error = ddp_rs::error::DDPError;
    type Color = RGB8;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        self.connection
            .write(
                &iterator
                    .into_iter()
                    .map(|c| c.into())
                    .flat_map(|rgb| [rgb.r, rgb.g, rgb.b].into_iter())
                    .collect::<Vec<u8>>(),
            )
            .map(drop)
    }
}
