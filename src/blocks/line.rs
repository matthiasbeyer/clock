use micromath::F32Ext;
use smart_leds::RGB8;

use super::Block;
use crate::data::Buffer;

pub struct Line {
    start: (usize, usize),
    end: (usize, usize),
    color: RGB8,
}

impl Line {
    pub fn new(start: (usize, usize), end: (usize, usize), color: RGB8) -> Self {
        Self { start, end, color }
    }
}

impl Block for Line {
    fn render_to_buffer<const X: usize, const Y: usize>(&self, buffer: &mut Buffer<X, Y>) {
        let distance_x = self.end.0 as f32 - self.start.0 as f32;
        let distance_y = self.end.1 as f32 - self.start.1 as f32;

        defmt::debug!("distance_x = {}, distance_y = {}", distance_x, distance_y);

        let distance_leds = (distance_x * distance_x + distance_y * distance_y).sqrt();
        defmt::debug!("distance_leds = {}", distance_leds);

        let mut cur_led = (self.start.0 as f32, self.start.1 as f32);

        for _ in 0..=distance_leds.ceil() as usize {
            defmt::debug!(
                "setting ({} / {}, {} / {})",
                cur_led.0,
                cur_led.0 as usize,
                cur_led.1,
                cur_led.1 as usize
            );
            buffer.set(cur_led.0 as usize, cur_led.1 as usize, self.color);
            cur_led.0 += distance_x / distance_leds;
            cur_led.1 += distance_y / distance_leds;
        }
    }
}
