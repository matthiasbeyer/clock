use smart_leds::RGB8;

use super::provider::Provider;

pub struct Rainbow {
    r: RgbSetting,
    g: RgbSetting,
    b: RgbSetting,
    mask: [bool; 3],
    color: RGB8,
}

struct RgbSetting {
    min: u8,
    max: u8,
    current: u8,
    increment: u8,
    count_mode: CountMode,
}

impl RgbSetting {
    fn new(min: u8, max: u8, increment: u8) -> Self {
        Self {
            max,
            min,
            current: min,
            increment,
            count_mode: CountMode::default(),
        }
    }

    fn is_at_max(&self) -> bool {
        self.current == self.max
    }

    fn is_at_min(&self) -> bool {
        self.current == self.min
    }

    fn advance(&mut self) {
        match self.count_mode {
            CountMode::Up => {
                self.current = self.current.saturating_add(self.increment);
                if self.current >= self.max {
                    self.current = self.max;
                }
            }
            CountMode::Down => {
                self.current = self.current.saturating_sub(self.increment);
                if self.current <= self.min {
                    self.current = self.min;
                }
            }
        }

        if self.current == self.max || self.current == self.min {
            self.count_mode.reverse()
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
enum CountMode {
    #[default]
    Up,
    Down,
}

impl CountMode {
    fn reverse(&mut self) {
        match self {
            CountMode::Up => *self = CountMode::Down,
            CountMode::Down => *self = CountMode::Up,
        }
    }
}

impl Rainbow {
    fn advance_mask(&mut self) {
        self.mask.rotate_right(1);
    }

    fn compute_next_setting(&mut self) {
        let advance_mask = [&mut self.r, &mut self.g, &mut self.b]
            .into_iter()
            .zip(self.mask.iter())
            .filter(|(_setting, mask)| **mask)
            .map(|(setting, _)| setting)
            .next()
            .map(|s| {
                s.advance();
                s.is_at_max() || s.is_at_min()
            })
            .unwrap_or(false);

        if advance_mask {
            self.advance_mask()
        }
    }
}

impl Provider for Rainbow {
    fn provide_next(&mut self) -> smart_leds::RGB8 {
        self.compute_next_setting();

        self.color.r = self.r.current;
        self.color.g = self.g.current;
        self.color.b = self.b.current;

        self.color.clone()
    }
}

impl Rainbow {
    pub fn new(
        (min_r, min_g, min_b): (u8, u8, u8),
        (max_r, max_g, max_b): (u8, u8, u8),
        increment: u8,
    ) -> Self {
        Self {
            r: RgbSetting::new(min_r, max_r, increment),
            g: RgbSetting::new(min_g, max_g, increment),
            b: RgbSetting::new(min_b, max_b, increment),
            mask: [true, false, false],
            color: RGB8::default(),
        }
    }
}
