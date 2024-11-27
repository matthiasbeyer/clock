use super::Program;

pub struct Duration {
    start_time: embassy_time::Instant,
}

impl Duration {
    pub fn new() -> Self {
        Self {
            start_time: embassy_time::Instant::now(),
        }
    }
}

impl Program for Duration {
    const TICKER_DURATION: embassy_time::Duration::from_secs(1);

    async fn tick(&mut self) {
        // empty
    }

    async fn render<const X: usize, const Y: usize>(&self, databuf: &mut crate::data::Buffer<X, Y>) {
        todo!()
    }
}
