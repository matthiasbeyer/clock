use crate::data::Buffer;

pub struct Ascii {
    chr: char,
}

impl Ascii {
    pub fn new(chr: char) -> Self {
        Self { chr }
    }

    pub fn render_to<const X: usize, const Y: usize>(
        &self,
        buffer: &mut Buffer<X, Y>,
        x: usize,
        y: usize,
    ) {
        unimplemented!()
    }
}
