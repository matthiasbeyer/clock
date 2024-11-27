use smart_leds::RGB8;

use super::Program;
use crate::blocks::text::Text;
use crate::blocks::Block;

pub struct Duration {
    start_time: embassy_time::Instant,
    color: RGB8,
}

impl Duration {
    pub fn new(color: RGB8) -> Self {
        Self {
            start_time: embassy_time::Instant::now(),
            color,
        }
    }
}

impl Program for Duration {
    const TICKER_DURATION: embassy_time::Duration = embassy_time::Duration::from_secs(1);

    async fn tick(&mut self) {
        // empty
    }

    async fn render<const X: usize, const Y: usize>(
        &self,
        databuf: &mut crate::data::Buffer<X, Y>,
    ) {
        crate::blocks::clear::Clear.render_to_buffer(databuf);
        let duration_secs = embassy_time::Instant::now()
            .saturating_duration_since(self.start_time)
            .as_secs();
        let dur_min = (duration_secs / 60) as u8;
        let dur_sec = (duration_secs % 60) as u8;

        let ss = crate::utils::stackstr!(5, "{:02}:{:02}", dur_min, dur_sec);

        let text = Text::new(ss.as_str(), (1, 1), self.color);
        text.render_to_buffer(databuf);
    }
}
