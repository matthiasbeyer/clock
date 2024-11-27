use embassy_time::Duration;
use smart_leds::RGB8;

use super::Program;
use crate::data::Buffer;
use crate::NUM_LEDS_X;
use crate::NUM_LEDS_Y;

#[derive(Default)]
pub struct RunningLight {
    x_offset: usize,
    y_offset: usize,
    previous: (usize, usize),
    color: RGB8,
}

impl RunningLight {
    pub fn new(color: RGB8) -> Self {
        Self {
            x_offset: 0,
            y_offset: 0,
            previous: (0, 0),
            color,
        }
    }
}

impl Program for RunningLight {
    const TICKER_DURATION: Duration = Duration::from_millis(100);

    async fn tick(&mut self) {
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

    async fn render<const X: usize, const Y: usize>(&self, buffer: &mut Buffer<X, Y>) {
        buffer.set(self.previous.0, self.previous.1, RGB8::default());
        buffer.set(self.x_offset, self.y_offset, self.color);
    }
}
