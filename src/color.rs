use embedded_graphics::pixelcolor::Rgb888;

pub struct ColorProvider {
    iter: ColorIter,
    set_color: Option<SetColor>,
    last_cycle_time: embassy_time::Instant,
}

struct SetColor {
    set_time: embassy_time::Instant,
    color: [u8; 3],
    duration: embassy_time::Duration,
}

impl ColorProvider {
    pub fn new(iter: ColorIter) -> Self {
        Self {
            iter,
            set_color: None,
            last_cycle_time: embassy_time::Instant::now(),
        }
    }

    pub fn set_color_for(&mut self, color: [u8; 3], duration: embassy_time::Duration) {
        self.set_color = Some(SetColor {
            set_time: embassy_time::Instant::now(),
            color,
            duration,
        });
    }
}

impl core::iter::Iterator for ColorProvider {
    type Item = Rgb888;

    fn next(&mut self) -> Option<Self::Item> {
        self.last_cycle_time = embassy_time::Instant::now();

        if let Some(set_color) = self.set_color.as_ref() {
            if set_color.set_time.elapsed() > set_color.duration {
                self.set_color = None;
            }
        }

        if let Some(set_color) = self.set_color.as_ref() {
            return Some(Rgb888::new(
                set_color.color[0],
                set_color.color[1],
                set_color.color[2],
            ));
        }

        self.iter.next()
    }
}

impl crate::render::Renderable for ColorProvider {
    fn get_next_cycle_time(&self) -> embassy_time::Instant {
        let next_iter_cycle_time = self.iter.get_next_cycle_time();

        if let Some(set_color_dur) = self.set_color.as_ref() {
            let next_set_color_cycle_time = set_color_dur.set_time + set_color_dur.duration;
            return next_iter_cycle_time.min(next_set_color_cycle_time);
        }

        next_iter_cycle_time
    }

    fn needs_cycle(&self) -> bool {
        if let Some(set_color) = self.set_color.as_ref() {
            return self.last_cycle_time.elapsed() > set_color.duration;
        }

        self.iter.needs_cycle()
    }
}

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
