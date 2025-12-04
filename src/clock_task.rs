use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use rgb::RGB8;
use smart_leds_matrix::layout::Layout;
use smart_leds_matrix::SmartLedMatrix;
use smart_leds_trait::SmartLedsWrite;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::config::Font;

pub struct ClockTask<T, L, const SIZE: usize>
where
    T: SmartLedsWrite,
    L: Layout,
    <T as SmartLedsWrite>::Color: From<RGB8>,
{
    interval: std::time::Duration,
    running: Arc<AtomicBool>,
    cancellation_token: CancellationToken,
    matrix: Arc<Mutex<SmartLedMatrix<T, L, SIZE>>>,
    time_offset: embedded_graphics::prelude::Point,
    time_font: Font,
}

impl<T, L, const SIZE: usize> ClockTask<T, L, SIZE>
where
    T: SmartLedsWrite,
    L: Layout,
    <T as SmartLedsWrite>::Color: From<RGB8>,
    crate::error::Error: From<<T as SmartLedsWrite>::Error>,
{
    pub fn new(
        running: Arc<AtomicBool>,
        cancellation_token: CancellationToken,
        matrix: Arc<Mutex<SmartLedMatrix<T, L, SIZE>>>,
        config: &crate::config::Config,
    ) -> Self {
        Self {
            interval: config.display.interval,
            running,
            cancellation_token,
            matrix,
            time_offset: embedded_graphics::prelude::Point::new(
                config.display.time_offset_x.into(),
                config.display.time_offset_y.into(),
            ),
            time_font: config.display.time_font,
        }
    }

    pub fn run(self) -> impl std::future::Future<Output = Result<(), crate::error::Error>> {
        let mut render_interval = tokio::time::interval(self.interval);
        let time_display_format = time::format_description::parse("[hour]:[minute]").unwrap();
        async move {
            let font = self.time_font.into();
            let mut clock_rainbow_style = crate::util::rainbow_color_iterator().map(|color| {
                embedded_graphics::mono_font::MonoTextStyle::new(&font, color)
            });

            let mut last_rendered_str = None;

            loop {
                let Some(_tick) = self
                    .cancellation_token
                    .run_until_cancelled(render_interval.tick())
                    .await
                else {
                    tracing::info!("Ending render interval");
                    break;
                };

                if self.running.load(std::sync::atomic::Ordering::Relaxed) {
                    let mut matrix = self.matrix.lock().await;

                    let time = time::OffsetDateTime::now_local()
                        .map_err(crate::error::Error::TimeOffset)?;

                    let time_str = time
                        .format(&time_display_format)
                        .map_err(crate::error::Error::TimeFormatting)?;

                    if last_rendered_str.is_some_and(|s| s != time_str) {
                        matrix
                            .clear(embedded_graphics::pixelcolor::Rgb888::default())
                            .unwrap();
                        matrix.flush()?;
                    }

                    // Draw text to the buffer
                    Text::new(&time_str, self.time_offset, clock_rainbow_style.next().unwrap())
                        .draw(&mut *matrix)
                        .unwrap();

                    matrix.flush()?;
                    tracing::trace!(?time_str, "Rendered clock");

                    last_rendered_str = Some(time_str);
                }
            }
            Ok(())
        }
    }
}
