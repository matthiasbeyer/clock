use super::Block;

pub struct Clear;

impl Block for Clear {
    fn render_to_buffer<const X: usize, const Y: usize>(
        &self,
        buffer: &mut crate::data::Buffer<X, Y>,
    ) {
        for x in 0..X {
            for y in 0..Y {
                buffer.set(x, y, smart_leds::RGB8::default());
            }
        }
    }
}
