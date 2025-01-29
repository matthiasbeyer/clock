#![no_std]
#![no_main]

use blocks::Block;
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
use programs::Program;
use smart_leds::RGB8;

mod blocks;
mod color;
mod data;
mod programs;
mod tab;
mod utils;

pub const NUM_LEDS: usize = 512;
pub const NUM_LEDS_X: usize = 32;
pub const NUM_LEDS_Y: usize = 16;

const RGB_WHITE: RGB8 = RGB8::new(10, 10, 10);

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

    defmt::info!("Starting");

    let mut buffer = Buffer::default();

    let program = PioWs2812Program::new(&mut common);
    let mut leds = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &program);

    let color_provider = {
        let mins = (10, 10, 10);
        let maxs = (20, 20, 20);
        let increment = 10;
        let inner = crate::color::rainbow::Rainbow::new(mins, maxs, increment);
        crate::color::konst::ConstNColor::<4>::from_provider(inner)
    };

    let programs = Programs {
        duration_time: ProgramWithState::new(crate::programs::duration::Duration::new(
            color_provider,
            true,
        )),
        running_light: ProgramWithState::new(crate::programs::running_light::RunningLight::new({
            let red = 10;
            let green = 10;
            let blue = 10;
            RGB8::new(red, green, blue)
        })),
    };

    let mut ticker = Ticker::every(embassy_time::Duration::from_secs(1));
    programs.run(&mut ticker, &mut buffer, &mut leds).await
}

struct ProgramWithState<P>
where
    P: Program,
{
    program: P,
    state: P::State,
}

impl<P> ProgramWithState<P>
where
    P: Program,
{
    fn new(program: P) -> Self {
        Self {
            program,
            state: P::State::default(),
        }
    }
}

struct Programs<CP>
where
    CP: crate::color::provider::Provider,
{
    // clock: crate::programs::clock::Clock,
    duration_time: ProgramWithState<crate::programs::duration::Duration<CP>>,
    running_light: ProgramWithState<crate::programs::running_light::RunningLight>,
}

impl<CP> Programs<CP>
where
    CP: crate::color::provider::Provider,
{
    async fn run<'d, P: Instance, const S: usize>(
        mut self,
        ticker: &mut Ticker,
        buffer: &mut Buffer,
        leds: &mut PioWs2812<'d, P, S, NUM_LEDS>,
    ) {
        loop {
            self.duration_time
                .program
                .render(buffer, &mut self.duration_time.state)
                .await;
            render(leds, &buffer).await;
            ticker.next().await;
        }
    }
}

async fn render<'d, P, const S: usize>(ws2812: &mut PioWs2812<'d, P, S, NUM_LEDS>, buffer: &Buffer)
where
    P: Instance,
{
    let mut intermediate_buffer: [RGB8; NUM_LEDS] = [RGB8::default(); NUM_LEDS];
    buffer.render_to_continuous_buffer::<{ NUM_LEDS }>(&mut intermediate_buffer);
    ws2812.write(&intermediate_buffer).await;
}
