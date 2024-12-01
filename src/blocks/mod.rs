pub mod character;
pub mod clear;
pub mod line;
pub mod text;
pub mod xy;

pub trait Block {
    fn render_to_buffer<const X: usize, const Y: usize>(
        &mut self,
        buffer: &mut crate::data::Buffer<X, Y>,
    );
}
