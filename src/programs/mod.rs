pub mod led_n;
pub mod running_light;
pub mod duration;

pub trait Program {
    const TICKER_DURATION: embassy_time::Duration;

    #[inline]
    fn ticker_duration(&self) -> embassy_time::Duration {
        Self::TICKER_DURATION
    }

    async fn tick(&mut self);
    async fn render<const X: usize, const Y: usize>(&self, databuf: &mut crate::data::Buffer<X, Y>);
}

