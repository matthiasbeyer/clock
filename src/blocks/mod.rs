pub mod character;
pub mod line;
pub mod xy;

pub trait Block {
    fn render_to_buffer<const X: usize, const Y: usize>(
        &self,
        buffer: &mut crate::data::Buffer<X, Y>,
    );
}
