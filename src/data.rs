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

pub struct OutputBuffer {
    buf: [RGB8; crate::NUM_LEDS],
}

impl OutputBuffer {
    pub fn new() -> Self {
        Self {
            buf: [RGB8::default(); crate::NUM_LEDS],
        }
    }

    fn is_valid_pixel<C>(pixel: &embedded_graphics::Pixel<C>) -> bool
    where
        C: embedded_graphics::prelude::PixelColor,
    {
        pixel.0.y >= 0
            && ((pixel.0.y as usize) < crate::NUM_LEDS_Y)
            && pixel.0.x >= 0
            && ((pixel.0.x as usize) < crate::NUM_LEDS_X)
    }

    pub async fn render_into<'d, P, const S: usize>(
        &self,
        ws2812: &mut embassy_rp::pio_programs::ws2812::PioWs2812<'d, P, S, { crate::NUM_LEDS }>,
    ) where
        P: embassy_rp::pio::Instance,
    {
        ws2812.write(&self.buf).await;
    }
}

impl embedded_graphics::prelude::Dimensions for OutputBuffer {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        embedded_graphics::primitives::Rectangle {
            top_left: embedded_graphics::prelude::Point { x: 0, y: 0 },
            size: embedded_graphics::prelude::Size {
                width: 32,
                height: 16,
            },
        }
    }
}

impl embedded_graphics::prelude::DrawTarget for OutputBuffer {
    type Color = embedded_graphics::pixelcolor::Rgb888;

    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for pixel in pixels.into_iter().filter(Self::is_valid_pixel) {
            let y = pixel.0.y;
            let x = pixel.0.x;

            if y < 0 || x < 0 {
                continue;
            }

            let pos = crate::tab::X_Y_TO_N_MAPPING_TABLE[y as usize][x as usize];
            self.buf[pos] = rgb888_to_rgb8(pixel.1);
        }
        Ok(())
    }
}

fn rgb888_to_rgb8(rgb888: embedded_graphics::pixelcolor::Rgb888) -> RGB8 {
    use embedded_graphics::prelude::RgbColor;
    RGB8::new(rgb888.r(), rgb888.g(), rgb888.b())
}
