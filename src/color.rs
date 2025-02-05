use embedded_graphics::pixelcolor::Rgb888;

#[derive(Debug, Clone)]
pub struct ColorIter {
    rgb: [u8; 3],
    idx: usize,
    max_intensity: u8,
    delay: u8,
    delay_state: u8,
}

impl ColorIter {
    pub const fn new(max_intensity: u8) -> Self {
        Self {
            rgb: [max_intensity, 0, 0],
            idx: 1,
            max_intensity,
            delay: 0,
            delay_state: 0,
        }
    }

    pub const fn with_delay(mut self, delay: u8) -> Self {
        self.delay = delay;
        self
    }
}

impl core::iter::Iterator for ColorIter {
    type Item = Rgb888;

    fn next(&mut self) -> Option<Self::Item> {
        if self.delay_state != 0 {
            self.delay_state -= 1;
            return Some(Rgb888::new(self.rgb[0], self.rgb[1], self.rgb[2]));
        } else {
            self.delay_state = self.delay;
        }

        if self.rgb[self.idx] != self.max_intensity {
            self.rgb[self.idx] += 1;
        } else {
            let prev_idx = if self.idx == 0 { 2 } else { self.idx - 1 };

            if self.rgb[prev_idx] != 0 {
                self.rgb[prev_idx] -= 1
            } else {
                self.idx += 1;
                if self.idx == 3 {
                    self.idx = 0;
                }
            }
        }

        Some(Rgb888::new(self.rgb[0], self.rgb[1], self.rgb[2]))
    }
}
