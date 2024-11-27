use smart_leds::RGB8;

pub struct Buffer<const X: usize, const Y: usize> {
    buffer: [[RGB8; X]; Y],
}

impl<const X: usize, const Y: usize> Default for Buffer<X, Y> {
    fn default() -> Self {
        Self {
            buffer: [[RGB8::default(); X]; Y],
        }
    }
}

const fn buffer_full_size<const X: usize, const Y: usize>() -> usize {
    X * Y
}

impl<const X: usize, const Y: usize> Buffer<X, Y> {
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> &RGB8 {
        &self.buffer[y][X - 1 - x]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, rgb: RGB8) {
        defmt::debug!(
            "Setting buffer ({}, {}) = ({}, {}, {})",
            y,
            x,
            rgb.r,
            rgb.g,
            rgb.b
        );
        self.buffer[Y - 1 - y][X - 1 - x] = rgb;
    }

    pub fn render_to_continuous_buffer<const CONT_SIZE: usize>(&self, buf: &mut [RGB8; CONT_SIZE]) {
        if CONT_SIZE != X * Y {
            panic!("Buffer not expected size")
        }

        for y in 0..Y {
            for x in 0..X {
                let pos = crate::tab::X_Y_TO_N_MAPPING_TABLE[y][x];
                buf[pos] = *self.get(x, y);
            }
        }
    }
}
