use smart_leds::RGB8;

use super::provider::Provider;

pub struct ConstColor {
    color: RGB8,
}

impl Provider for ConstColor {
    fn provide_next(&mut self) -> smart_leds::RGB8 {
        self.color.clone()
    }
}

impl ConstColor {
    pub fn new(color: RGB8) -> Self {
        Self { color }
    }
}
