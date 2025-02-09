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
use panic_probe as _;
use render::RenderToDisplay;
use render::Renderable;

mod bounding_box;
mod clock;
mod color;
mod mapping;
mod output;
mod render;
mod util;
mod ntp;

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

    let mut color_iter = crate::color::ColorIter::new(10, embassy_time::Duration::from_secs(1));

    let mut color = color_iter.next().unwrap();
    let mut clock = crate::clock::Timer::new(
        embassy_time::Instant::now(),
        embassy_time::Duration::from_secs(1),
    );
    let mut border = crate::bounding_box::BoundingBox::new();

    loop {
        let cycle_start_time = embassy_time::Instant::now();

        if color_iter.needs_cycle() {
            color = color_iter.next().unwrap();
        }

        let mut display = output::OutputBuffer::new();

        if clock.needs_cycle() {
            border.render_to_display(&mut display, color);
            clock.render_to_display(&mut display, color);
        }

        display.render_into(&mut leds).await;

        let min_cycle_duration = [
            color_iter.get_next_cycle_time(),
            clock.get_next_cycle_time(),
        ]
        .into_iter()
        .min()
        .unwrap_or_else(embassy_time::Instant::now);

        let cycle_duration = embassy_time::Instant::now().duration_since(cycle_start_time);

        if let Some(sleep_until) = min_cycle_duration.checked_sub(cycle_duration) {
            if let Some(sleep_time) =
                sleep_until.checked_duration_since(embassy_time::Instant::now())
            {
                embassy_time::Timer::after(sleep_time).await
            }
        }
    }
}
