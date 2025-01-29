use super::Program;
use crate::blocks::text::Text;
use crate::blocks::Block;

pub struct Duration<CP> {
    color: CP,
    colored_chars: bool,
}

impl<CP> Duration<CP> {
    pub fn new(color: CP, colored_chars: bool) -> Self {
        Self {
            color,
            colored_chars,
        }
    }
}

pub struct DurationState {
    start_time: embassy_time::Instant,
}

impl Default for DurationState {
    fn default() -> Self {
        Self {
            start_time: embassy_time::Instant::now(),
        }
    }
}

impl super::ProgramState for DurationState {}

impl DurationState {
    fn get_duration(&self) -> embassy_time::Duration {
        embassy_time::Instant::now().saturating_duration_since(self.start_time)
    }
}

impl<CP> Program for Duration<CP>
where
    CP: crate::color::provider::Provider,
{
    const TICKER_DURATION: embassy_time::Duration = embassy_time::Duration::from_secs(1);

    type State = DurationState;

    async fn render<const X: usize, const Y: usize>(
        &mut self,
        databuf: &mut crate::data::Buffer<X, Y>,
        state: &mut Self::State,
    ) {
        crate::blocks::clear::Clear.render_to_buffer(databuf);
        let duration = state.get_duration();
        let duration = if duration >= embassy_time::Duration::from_secs(99 * 60 + 59) {
            state.start_time = embassy_time::Instant::now();
            state.get_duration()
        } else {
            duration
        };
        let duration_secs = duration.as_secs();
        let dur_min = (duration_secs / 60) as u8;
        let dur_sec = (duration_secs % 60) as u8;

        let ss = crate::utils::stackstr!(5, "{:02}:{:02}", dur_min, dur_sec);

        if self.colored_chars {
            let mut text =
                crate::blocks::text::TextColored::new(ss.as_str(), (5, 5), &mut self.color);
            text.render_to_buffer(databuf);
        } else {
            let color = self.color.provide_next();
            let mut text = Text::new(ss.as_str(), (1, 1), color);
            text.render_to_buffer(databuf);
        }
    }
}
