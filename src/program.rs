use defmt::debug;
use embassy_time::Duration;

use crate::data::Buffer;
use crate::LED_OFF;
use crate::LED_WHITE;
use crate::NUM_LEDS_X;
use crate::NUM_LEDS_Y;

pub trait Program {
    const TICKER_DURATION: Duration;

    async fn tick(&mut self);
    async fn render(&self, databuf: &mut Buffer);
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

    async fn render(&self, databuf: &mut Buffer) {
        databuf[self.previous.1][self.previous.0] = LED_OFF;
        databuf[self.y_offset][self.x_offset] = LED_WHITE;
    }
}
