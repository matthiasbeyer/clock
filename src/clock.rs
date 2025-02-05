use embedded_graphics::prelude::Dimensions;
use embedded_graphics::Drawable;

pub struct Timer {
    start_time: embassy_time::Instant,
    render_speed: embassy_time::Duration,
    last_cycle_time: embassy_time::Instant,
}

impl Timer {
    pub fn new(start_time: embassy_time::Instant, render_speed: embassy_time::Duration) -> Self {
        Self {
            start_time,
            render_speed,
            last_cycle_time: start_time,
        }
    }
}

impl crate::render::Renderable for Timer {
    fn get_next_cycle_time(&self) -> embassy_time::Instant {
        self.last_cycle_time
            .checked_add(self.render_speed)
            .unwrap_or_else(embassy_time::Instant::now)
    }

    fn needs_cycle(&self) -> bool {
        self.last_cycle_time.elapsed() >= self.render_speed
    }
}

impl crate::render::RenderToDisplay for Timer {
    fn render_to_display(
        &mut self,
        display: &mut crate::output::OutputBuffer,
        color: embedded_graphics::pixelcolor::Rgb888,
    ) {
        // Draw centered text.
        let time_text = {
            let duration = embassy_time::Instant::now().duration_since(self.start_time);
            let duration = if duration >= embassy_time::Duration::from_secs(99 * 60 + 59) {
                self.start_time = embassy_time::Instant::now();
                embassy_time::Instant::now().duration_since(self.start_time)
            } else {
                duration
            };

            let duration_secs = duration.as_secs();
            let dur_min = (duration_secs / 60) as u8;
            let dur_sec = (duration_secs % 60) as u8;

            crate::stackstr!(5, "{:02}:{:02}", dur_min, dur_sec)
        };

        let character_style = embedded_graphics::mono_font::MonoTextStyle::new(
            &embedded_graphics::mono_font::ascii::FONT_5X8,
            color,
        );

        embedded_graphics::text::Text::with_alignment(
            time_text.as_str(),
            display.bounding_box().center() + embedded_graphics::prelude::Point::new(0, 3),
            character_style,
            embedded_graphics::text::Alignment::Center,
        )
        .draw(display)
        .unwrap();
    }
}
