use embassy_time::Duration;
use embassy_time::Instant;
use embedded_graphics::prelude::Dimensions;
use embedded_graphics::Drawable;
use sntpc::NtpResult;

pub struct Clock {
    ntp: NtpResult,
    last_ntp: Instant,
    is_first_run: bool,
}

impl Clock {
    pub fn new(ntp: NtpResult, last_ntp: Instant) -> Self {
        Self {
            ntp,
            last_ntp,
            is_first_run: true,
        }
    }

    pub fn set_system_time(&mut self, ntp: sntpc::NtpResult, last_ntp: Instant) {
        self.ntp = ntp;
        self.last_ntp = last_ntp;
    }
}

impl crate::render::Renderable for Clock {
    fn get_next_cycle_time(&self) -> embassy_time::Instant {
        self.last_ntp
            .checked_add(embassy_time::Duration::from_secs(60))
            .unwrap_or_else(embassy_time::Instant::now)
    }

    fn needs_cycle(&self) -> bool {
        self.last_ntp.elapsed() >= embassy_time::Duration::from_secs(60) || self.is_first_run
    }
}

impl crate::render::RenderToDisplay for Clock {
    fn render_to_display(
        &mut self,
        display: &mut crate::output::OutputBuffer,
        color: embedded_graphics::pixelcolor::Rgb888,
    ) {
        self.is_first_run = false;
        // Draw centered text.
        let time_text = {
            let current_time = Duration::from_secs(self.ntp.sec().into())
                .checked_add(Instant::now().duration_since(self.last_ntp))
                .unwrap();

            let current_time_secs = current_time.as_secs();
            let seconds_today = current_time_secs % (60 * 60 * 24);
            let curr_hour = seconds_today / 3600;
            let curr_min = seconds_today % 3600 / 60;

            defmt::debug!("Building time: {}:{}", curr_hour, curr_min);
            crate::stackstr!(5, "{:02}:{:02}", curr_hour, curr_min)
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
