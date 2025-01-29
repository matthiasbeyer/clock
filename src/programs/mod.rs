pub mod duration;
pub mod led_n;
pub mod running_light;

pub trait Program {
    const TICKER_DURATION: embassy_time::Duration;

    type State: ProgramState;

    #[inline]
    fn ticker_duration(&self) -> embassy_time::Duration {
        Self::TICKER_DURATION
    }

    async fn render<const X: usize, const Y: usize>(
        &mut self,
        databuf: &mut crate::data::Buffer<X, Y>,
        state: &mut Self::State,
    );
}

pub trait ProgramState: Default {
}
