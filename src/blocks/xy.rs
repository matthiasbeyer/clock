use smart_leds::RGB8;

use super::Block;

#[derive(Default)]
pub struct XY {
    x: usize,
    y: usize,
    is_on: bool,
    color: RGB8,
}

impl XY {
    pub fn new(x: usize, y: usize, color: RGB8) -> Self {
        Self {
            x,
            y,
            is_on: false,
            color,
        }
    }

    pub fn color(&self) -> RGB8 {
        self.color
    }

    pub fn set_color(&mut self, color: RGB8) {
        self.color = color;
    }
}

impl Block for XY {
    fn render_to_buffer<const X: usize, const Y: usize>(
        &mut self,
        buffer: &mut crate::data::Buffer<X, Y>,
    ) {
        let color = if self.is_on {
            RGB8::default()
        } else {
            self.color
        };
        defmt::debug!("Coloring LED ({}, {})", self.x, self.y);
        buffer.set(self.x, self.y, color);
    }
}
