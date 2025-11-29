use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use smart_leds_matrix::layout::Rectangular;
use smart_leds_matrix::SmartLedMatrix;

mod cli;
mod config;
mod error;
mod event;
mod logging;
mod mqtt;
mod systemd;
mod util;
mod writer;

#[tokio::main(flavor = "current_thread")]
async fn main() -> color_eyre::eyre::Result<()> {
    setup_panic();
    color_eyre::install().map_err(crate::error::Error::InstallingColorEyre)?;
    let cli = <crate::cli::Cli as clap::Parser>::parse();
    crate::logging::setup(cli.verbosity);
    let cfg = crate::config::Config::load(&cli.config).await?;

    match cli.command {
        cli::Command::Run => {
            run(cli, cfg).await?;
        }
        cli::Command::VerifyConfig => {
            tracing::info!("Configuration verified");
        }
    }

    Ok(())
}

fn setup_panic() {
    human_panic::setup_panic!(human_panic::Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
    .authors("Matthias Beyer <mail@beyermatthias.de>"));
}

async fn run(
    _cli: crate::cli::Cli,
    config: crate::config::Config,
) -> Result<(), crate::error::Error> {
    let ddp_connection = ddp_rs::connection::DDPConnection::try_new(
        format!("{}:{}", config.display.host, config.display.port),
        ddp_rs::protocol::PixelConfig::default(), // Default is RGB, 8 bits ber channel
        ddp_rs::protocol::ID::Default,
        std::net::UdpSocket::bind(format!("0.0.0.0:{}", config.display.udp_port))
            .map_err(crate::error::Error::UDPBind)?,
    )?;

    let writer = writer::Writer::new(ddp_connection);
    let mut matrix = SmartLedMatrix::<_, _, { 32 * 16 }>::new(writer, Rectangular::new(32, 16));
    matrix.set_brightness(config.display.initial_brightness.clamp(0, 100));
    matrix
        .clear(embedded_graphics::pixelcolor::Rgb888::default())
        .unwrap();
    matrix.flush()?;

    let (event_sender, mut event_receiver) = tokio::sync::mpsc::channel::<event::Event>(100);
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    tokio::task::spawn({
        let mqtt_config = config.mqtt.clone();
        let cancellation_token = cancellation_token.clone();
        mqtt::run(mqtt_config, cancellation_token, event_sender)
    });

    let mut render_interval = tokio::time::interval(std::time::Duration::from_secs(1));
    let time_display_format = time::format_description::parse("[hour]:[minute]").unwrap();
    let time_font = config.display.time_font.into();
    let time_offset = Point::new(
        config.display.time_offset_x.into(),
        config.display.time_offset_y.into(),
    );
    let mut clock_rainbow_style =
        util::rainbow_color_iterator().map(|color| MonoTextStyle::new(&time_font, color));

    loop {
        tokio::select! {
            _ = render_interval.tick() => {
                let time = time::OffsetDateTime::now_local().map_err(error::Error::TimeOffset)?;
                let time_str = time.format(&time_display_format).map_err(error::Error::TimeFormatting)?;

                matrix.clear(embedded_graphics::pixelcolor::Rgb888::default()).unwrap();
                matrix.flush()?;

                // Draw text to the buffer
                Text::new(&time_str, time_offset, clock_rainbow_style.next().unwrap()).draw(&mut matrix).unwrap();
                matrix.flush()?;
            }

            event = event_receiver.recv() => {
                let Some(event) = event else { tracing::error!("Receiver closed"); break };

                match event {
                    event::Event::SetBrightness(brightness) => {
                        tracing::info!(?brightness, "Setting brightness");
                        matrix.set_brightness(brightness.clamp(5, 100));
                    },

                    event::Event::ShowText { duration_secs, text } => {
                        tracing::info!(?duration_secs, ?text, "Showing text");
                    },
                }
            }

            _ctrl_c = tokio::signal::ctrl_c() => {
                tracing::info!("Ctrl-C received, shutting down");
                cancellation_token.cancel();
                break
            }
        }
    }

    Ok(())
}
