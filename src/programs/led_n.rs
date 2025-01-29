use embassy_time::Duration;
use smart_leds::RGB8;

use super::Program;

#[derive(Default)]
pub struct LedN {
    color: RGB8,
    n: usize,
}

impl LedN {
    pub fn new(n: usize, color: RGB8) -> Self {
        Self { n, color }
    }

    pub fn color(&self) -> &RGB8 {
        &self.color
    }

    pub fn set_color(&mut self, color: RGB8) {
        self.color = color;
    }
}

#[derive(Default)]
pub struct LedNState {
    is_on: bool,
}

impl super::ProgramState for LedNState {}

impl Program for LedN {
    const TICKER_DURATION: Duration = Duration::from_secs(1);

    type State = LedNState;

    async fn render<const X: usize, const Y: usize>(
        &mut self,
        buffer: &mut crate::data::Buffer<X, Y>,
        state: &mut Self::State,
    ) {
        state.is_on = !state.is_on;
        let color = if state.is_on {
            RGB8::default()
        } else {
            self.color
        };

        let x = self.n % X;
        let y = self.n / Y;

        defmt::debug!("Coloring LED {} at ({}, {})", self.n, x, y);

        buffer.set(x, y, color);
    }
}
