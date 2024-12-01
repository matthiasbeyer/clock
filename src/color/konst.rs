use smart_leds::RGB8;

use super::provider::Provider;

pub struct ConstColor {
    color: RGB8,
}

impl Provider for ConstColor {
    fn provide_next(&mut self) -> smart_leds::RGB8 {
        self.color.clone()
    }
}

impl ConstColor {
    pub fn new(color: RGB8) -> Self {
        Self { color }
    }
}

pub struct ConstNColor<const N: usize> {
    color: [RGB8; N],
    idx: usize,
}

impl<const N: usize> ConstNColor<N> {
    pub const fn from_slice(color: [RGB8; N]) -> Self {
        Self { color, idx: 0 }
    }

    pub fn from_provider<CP>(mut provider: CP) -> Self
    where
        CP: Provider,
    {
        let mut color = [RGB8::default(); N];

        for idx in 0..N {
            color[idx] = provider.provide_next();
        }

        Self { color, idx: 0 }
    }
}

impl<const N: usize> Provider for ConstNColor<N> {
    fn provide_next(&mut self) -> smart_leds::RGB8 {
        let current = self.color[self.idx];
        self.idx += 1;
        if self.idx == N {
            self.idx = 0;
        }
        current
    }
}
