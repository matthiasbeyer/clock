use defmt::debug;
use embassy_time::Duration;

use crate::data::Buffer;
use crate::LED_OFF;
use crate::LED_WHITE;
use crate::NUM_LEDS_X;
use crate::NUM_LEDS_Y;

pub trait Program {
    const TICKER_DURATION: Duration;

    #[inline]
    fn ticker_duration(&self) -> Duration {
        Self::TICKER_DURATION
    }

    async fn tick(&mut self);
    async fn render<const X: usize, const Y: usize>(&self, databuf: &mut Buffer<X, Y>);
}

//pub struct Clock;
//
//impl Program for Clock {
//    const TICKER_DURATION: Duration = Duration::from_secs(1);
//
//    async fn tick(&mut self) {
//        todo!()
//    }
//
//    async fn render(&self, databuf: &mut Buffer) {
//        todo!()
//    }
//}

#[derive(Default)]
pub struct RunningLight {
    x_offset: usize,
    y_offset: usize,
    previous: (usize, usize),
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

        debug!("Setting (x, y) = ({}, {})", self.x_offset, self.y_offset);
    }

    async fn render<const X: usize, const Y: usize>(&self, buffer: &mut Buffer<X, Y>) {
        buffer.set(self.previous.0, self.previous.1, LED_OFF);
        buffer.set(self.x_offset, self.y_offset, LED_WHITE);
    }
}

#[derive(Default)]
pub struct LedN<const N: usize> {
    is_on: bool,
}

impl<const N: usize> Program for LedN<N> {
    const TICKER_DURATION: Duration = Duration::from_secs(1);

    async fn tick(&mut self) {
        self.is_on = !self.is_on;
    }

    async fn render<const X: usize, const Y: usize>(&self, buffer: &mut Buffer<X, Y>) {
        let color = if self.is_on { LED_WHITE } else { LED_OFF };

        let x = N % X;
        let y = N / Y;

        debug!("Coloring LED {} at ({}, {})", N, x, y);

        buffer.set(x, y, color);
    }
}

#[derive(Default)]
pub struct LedXY<const X: usize, const Y: usize> {
    is_on: bool,
}

impl<const X: usize, const Y: usize> Program for LedXY<X, Y> {
    const TICKER_DURATION: Duration = Duration::from_secs(1);

    async fn tick(&mut self) {
        self.is_on = !self.is_on;
    }

    async fn render<const SIZE_X: usize, const SIZE_Y: usize>(
        &self,
        buffer: &mut Buffer<SIZE_X, SIZE_Y>,
    ) {
        let color = if self.is_on { LED_WHITE } else { LED_OFF };
        debug!("Coloring LED ({}, {})", X, Y);
        buffer.set(X, Y, color);
    }
}
