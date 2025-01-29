use embassy_time::Duration;
use smart_leds::RGB8;

use super::Program;
use crate::data::Buffer;
use crate::NUM_LEDS_X;
use crate::NUM_LEDS_Y;

#[derive(Default)]
pub struct RunningLight {
    color: RGB8,
}

impl RunningLight {
    pub fn new(color: RGB8) -> Self {
        Self { color }
    }
}

#[derive(Default)]
pub struct RunningLightState {
    previous: (usize, usize),
    x_offset: usize,
    y_offset: usize,
}

impl super::ProgramState for RunningLightState {}

impl RunningLightState {
    fn tick(&mut self) {
        self.previous = (self.x_offset, self.y_offset);
        self.x_offset += 1;

        if self.x_offset == NUM_LEDS_X {
            self.x_offset = 0;
            self.y_offset += 1;
        }

        if self.y_offset == NUM_LEDS_Y {
            self.y_offset = 0;
        }

        defmt::debug!("Setting (x, y) = ({}, {})", self.x_offset, self.y_offset);
    }
}

impl Program for RunningLight {
    const TICKER_DURATION: Duration = Duration::from_millis(100);

    type State = RunningLightState;

    async fn render<const X: usize, const Y: usize>(
        &mut self,
        buffer: &mut Buffer<X, Y>,
        state: &mut Self::State,
    ) {
        state.tick();
        buffer.set(state.previous.0, state.previous.1, RGB8::default());
        buffer.set(state.x_offset, state.y_offset, self.color);
    }
}
