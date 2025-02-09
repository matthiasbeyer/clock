use core::net::IpAddr;
use core::net::SocketAddr;

use cyw43_pio::PioSpi;
use embassy_executor::Spawner;
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::PacketMetadata;
use embassy_net::udp::UdpSocket;
use embassy_net::StackResources;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::DMA_CH0;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::Pio;
use embassy_time::Duration;
use embassy_time::Timer;
use sntpc::NtpContext;
use sntpc::NtpTimestampGenerator;
use static_cell::StaticCell;

use crate::Irqs;

const NTP_SERVER: &str = "pool.ntp.org";

#[derive(Copy, Clone, Default)]
struct Timestamp {
    duration: Duration,
}

impl NtpTimestampGenerator for Timestamp {
    fn init(&mut self) {
        todo!()
    }

    fn timestamp_sec(&self) -> u64 {
        self.duration.as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        todo!()
    }
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

static NETWORK_STATE: StaticCell<cyw43::State> = StaticCell::new();

static FIRMWARE_FW: &[u8] = include_bytes!(env!("CYW43_FIRMWARE_BIN"));
static FIRMWARE_CLM: &[u8] = include_bytes!(env!("CYW43_FIRMWARE_CLM_BIN"));

async fn init_net(spawner: Spawner, p: embassy_rp::Peripherals) -> embassy_rp::Peripherals {
    let state = NETWORK_STATE.init(cyw43::State::new());

    let pwr = Output::new(p.PIN_23, embassy_rp::gpio::Level::Low);
    let cs = Output::new(p.PIN_25, embassy_rp::gpio::Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        cyw43_pio::DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, FIRMWARE_FW).await;

    spawner.spawn(cyw43_task(runner)).unwrap();

    p
}

#[embassy_executor::task]
async fn main_task(spawner: Spawner, p: &mut embassy_rp::Peripherals) {

    control.init(FIRMWARE_CLM).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Configure network stack
    let config = embassy_net::Config::dhcpv4(Default::default());

    // Init network stack
    let (stack, runner) =
        embassy_net::new(device, config, RESOURCES.init(StackResources::new()), 0);

    // Launch network task
    spawner.spawn(net_task(runner)).unwrap();

    // Wait for the tap interface to be up before continuing
    stack.wait_config_up().await;

    // Create UDP socket
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    socket.bind(123).unwrap();

    let context = NtpContext::new(Timestamp::default());

    let ntp_addrs = stack
        .dns_query(NTP_SERVER, DnsQueryType::A)
        .await
        .expect("Failed to resolve DNS");

    if ntp_addrs.is_empty() {
        defmt::error!("Failed to resolve DNS");
        return;
    }

    loop {
        let addr: IpAddr = ntp_addrs[0].into();
        let result = sntpc::get_time(SocketAddr::from((addr, 123)), &socket, context).await;

        match result {
            Ok(time) => {
                defmt::info!("Time: {:?}", time);
            }
            Err(e) => {
                defmt::error!("Error getting time: {:?}", e);
            }
        }

        Timer::after(Duration::from_secs(15)).await;
    }
}
