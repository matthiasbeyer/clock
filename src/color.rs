use embedded_graphics::pixelcolor::Rgb888;

#[derive(Debug, Clone)]
pub struct ColorIter {
    rgb: [u8; 3],
    idx: usize,
    max_intensity: u8,
    duration: embassy_time::Duration,
    last_cycle_time: embassy_time::Instant,
}

impl ColorIter {
    pub fn new(max_intensity: u8, duration: embassy_time::Duration) -> Self {
        Self {
            rgb: [max_intensity, 0, 0],
            idx: 1,
            max_intensity,
            duration,
            last_cycle_time: embassy_time::Instant::now(),
        }
    }
}

impl crate::render::Renderable for ColorIter {
    fn get_next_cycle_time(&self) -> embassy_time::Instant {
        self.last_cycle_time
            .checked_add(self.duration)
            .unwrap_or_else(embassy_time::Instant::now)
    }

    fn needs_cycle(&self) -> bool {
        self.last_cycle_time.elapsed() >= self.duration
    }
}

impl core::iter::Iterator for ColorIter {
    type Item = Rgb888;

    fn next(&mut self) -> Option<Self::Item> {
        self.last_cycle_time = embassy_time::Instant::now();

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
