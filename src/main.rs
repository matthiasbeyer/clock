#![no_std]
#![no_main]

use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::config::Config;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::InterruptHandler;
use embassy_rp::pio::Pio;
use embassy_rp::pio_programs::ws2812::PioWs2812;
use embassy_rp::pio_programs::ws2812::PioWs2812Program;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::Dimensions;
use embedded_graphics::prelude::Point;
use embedded_graphics::prelude::Primitive;
use embedded_graphics::prelude::WebColors;
use embedded_graphics::primitives::PrimitiveStyleBuilder;
use embedded_graphics::primitives::StrokeAlignment;
use embedded_graphics::text::Alignment;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use panic_probe as _;

mod color;
mod mapping;
mod output;
mod util;

pub const NUM_LEDS: usize = 512;
pub const NUM_LEDS_X: usize = 32;
pub const NUM_LEDS_Y: usize = 16;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let config = Config::default();
    let p = embassy_rp::init(config);

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    defmt::info!("Starting");

    let program = PioWs2812Program::new(&mut common);
    let mut leds = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &program);

    let mut color_iter = crate::color::ColorIter::new(10)
        .with_delay(10)
        .cycle();

    let mut start_time = embassy_time::Instant::now();

    loop {
        let color = color_iter.next().unwrap();
        let character_style =
            MonoTextStyle::new(&embedded_graphics::mono_font::ascii::FONT_5X8, color);
        let mut display = output::OutputBuffer::new();

        let border_stroke = PrimitiveStyleBuilder::new()
            .stroke_color(color)
            .stroke_width(1)
            .stroke_alignment(StrokeAlignment::Inside)
            .build();

        // Draw a px wide outline around the display.
        display
            .bounding_box()
            .into_styled(border_stroke)
            .draw(&mut display)
            .unwrap();

        // Draw centered text.
        let time_text = {
            let duration = embassy_time::Instant::now().duration_since(start_time);
            let duration = if duration >= embassy_time::Duration::from_secs(99 * 60 + 59) {
                start_time = embassy_time::Instant::now();
                embassy_time::Instant::now().duration_since(start_time)
            } else {
                duration
            };

            let duration_secs = duration.as_secs();
            let dur_min = (duration_secs / 60) as u8;
            let dur_sec = (duration_secs % 60) as u8;

            crate::stackstr!(5, "{:02}:{:02}", dur_min, dur_sec)
        };

        Text::with_alignment(
            time_text.as_str(),
            display.bounding_box().center() + Point::new(0, 3),
            character_style,
            Alignment::Center,
        )
        .draw(&mut display)
        .unwrap();

        display.render_into(&mut leds).await;
    }
}
