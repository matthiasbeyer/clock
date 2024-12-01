use smart_leds::RGB8;

use super::character::Character;
use super::Block;

pub struct Text<'s> {
    text: &'s str,
    color: RGB8,
    offset: (usize, usize),
}

impl<'s> Text<'s> {
    pub fn new(text: &'s str, offset: (usize, usize), color: RGB8) -> Self {
        Self {
            text,
            color,
            offset,
        }
    }
}

impl Block for Text<'_> {
    fn render_to_buffer<const X: usize, const Y: usize>(
        &mut self,
        buffer: &mut crate::data::Buffer<X, Y>,
    ) {
        let mut offset = self.offset;
        for chr in self.text.chars().map(|c| c.to_uppercase()).flatten() {
            let mut c = Character::new(chr, offset, self.color);
            c.render_to_buffer(buffer);

            offset.0 += c.width();
        }
    }
}

pub struct TextColored<'s, CP> {
    text: &'s str,
    color_provider: &'s mut CP,
    offset: (usize, usize),
}

impl<'s, CP> TextColored<'s, CP> {
    pub fn new(text: &'s str, offset: (usize, usize), color_provider: &'s mut CP) -> Self {
        Self {
            text,
            color_provider,
            offset,
        }
    }
}

impl<CP> Block for TextColored<'_, CP>
where
    CP: crate::color::provider::Provider,
{
    fn render_to_buffer<const X: usize, const Y: usize>(
        &mut self,
        buffer: &mut crate::data::Buffer<X, Y>,
    ) {
        let mut offset = self.offset;
        for chr in self.text.chars().map(|c| c.to_uppercase()).flatten() {
            let mut c = Character::new(chr, offset, self.color_provider.provide_next());
            c.render_to_buffer(buffer);

            offset.0 += c.width();
        }
    }
}
