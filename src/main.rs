#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::config::Config;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::Instance;
use embassy_rp::pio::InterruptHandler;
use embassy_rp::pio::Pio;
use embassy_rp::pio_programs::ws2812::PioWs2812;
use embassy_rp::pio_programs::ws2812::PioWs2812Program;
use embassy_time::Ticker;
use panic_probe as _;
use program::LedN;
use program::LedXY;
use smart_leds::RGB8;

use crate::program::Program;
use crate::program::RunningLight;

mod tab;
mod data;
mod program;

pub const NUM_LEDS: usize = 512;
pub const NUM_LEDS_X: usize = 32;
pub const NUM_LEDS_Y: usize = 16;

pub const LED_OFF: RGB8 = RGB8::new(0, 0, 0);
pub const LED_WHITE: RGB8 = RGB8::new(10, 10, 10);

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

type Buffer = crate::data::Buffer<NUM_LEDS_X, NUM_LEDS_Y>;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let config = Config::default();
    let p = embassy_rp::init(config);

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let mut buffer = Buffer::default();

    let program = PioWs2812Program::new(&mut common);
    let mut leds = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &program);
    let mut light = LedXY::<4, 3>::default();
    let mut ticker = Ticker::every(light.ticker_duration());
    loop {
        light.tick().await;
        light.render(&mut buffer).await;
        ticker.next().await;
        render(&mut leds, &buffer).await;
    }
}

async fn render<'d, P, const S: usize>(ws2812: &mut PioWs2812<'d, P, S, NUM_LEDS>, buffer: &Buffer)
where
    P: Instance,
{
    let mut intermediate_buffer: [RGB8; NUM_LEDS] = [LED_OFF; NUM_LEDS];
    buffer.render_to_continuous_buffer::<{NUM_LEDS}>(&mut intermediate_buffer);
    ws2812.write(&intermediate_buffer).await;
}
