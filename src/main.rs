use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use smart_leds_matrix::layout::Rectangular;
use smart_leds_matrix::SmartLedMatrix;
use url::Url;

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
    tracing::info!("Created DDP connection");

    let state_url = Url::try_from(format!("http://{}/json/state", config.display.host).as_ref())?;
    let effects_url = Url::try_from(format!("http://{}/json/eff", config.display.host).as_ref())?;

    let wled_client = reqwest::ClientBuilder::new()
        .gzip(true)
        .timeout(std::time::Duration::from_millis(5000u64))
        .build()
        .map_err(crate::error::Error::Reqwest)?;

    tracing::info!("Created WLED connection");

    wled_client
        .post(state_url.clone())
        .json(&wled_api_types::types::state::State {
            on: Some(true),
            ..Default::default()
        })
        .send()
        .await
        .inspect(|response| tracing::debug!(?response, "Successfully flushed state to WLED"))
        .inspect_err(|error| tracing::error!(?error, "WLED Client errored"))
        .map_err(crate::error::Error::Reqwest)?;

    tracing::info!("Booted WLED clock successfully");

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

    let mut render_interval = tokio::time::interval(config.display.interval);
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

                match event.event {
                    event::EventInner::TurnOn => {
                        wled_client
                            .post(state_url.clone())
                            .json(&wled_api_types::types::state::State {
                                on: Some(true),
                                bri: Some(config.display.initial_brightness.clamp(0, 100)),
                                tt: Some(10),
                                ..Default::default()
                            })
                            .send()
                            .await
                            .inspect(|response| tracing::debug!(?response, "Successfully flushed state to WLED"))
                            .inspect_err(|error| tracing::error!(?error, "WLED Client errored"))
        .map_err(crate::error::Error::Reqwest)?;
                        tracing::info!("Updated WLED state");

                        matrix.set_brightness(config.display.initial_brightness.clamp(0, 100));
                        matrix
                            .clear(embedded_graphics::pixelcolor::Rgb888::default())
                            .unwrap();
                        matrix.flush()?;
                    },

                    event::EventInner::TurnOff => {
                        wled_client
                            .post(state_url.clone())
                            .json(&wled_api_types::types::state::State {
                                on: Some(false),
                                ..Default::default()
                            })
                            .send()
                            .await
                            .inspect(|response| tracing::debug!(?response, "Successfully flushed state to WLED"))
                            .inspect_err(|error| tracing::error!(?error, "WLED Client errored"))
        .map_err(crate::error::Error::Reqwest)?;
                        tracing::info!("Updated WLED state");
                    },

                    event::EventInner::SetBrightness(brightness) => {
                        tracing::info!(?brightness, "Setting brightness");
                        matrix.set_brightness(brightness.clamp(5, 100));
                    },

                    event::EventInner::ShowText { duration_secs, text, x, y } => {
                        tracing::info!(?duration_secs, ?text, "Showing text");

                        let mut render_interval = tokio::time::interval(config.display.interval);
                        let start_time = std::time::Instant::now();
                        let duration_secs = std::time::Duration::from_secs(duration_secs.into());

                        let offset = Point::new(
                            x.into(),
                            y.into(),
                        );

                        matrix
                            .clear(embedded_graphics::pixelcolor::Rgb888::default())
                            .unwrap();

                        while start_time.elapsed() < duration_secs {
                            Text::new(&text, offset, clock_rainbow_style.next().unwrap()).draw(&mut matrix).unwrap();
                            matrix.flush()?;

                            let _ = render_interval.tick().await;
                        }
                    },

                    event::EventInner::ShowPreset { name, duration_s, c1, c2, c3 } => {
                        let effects = wled_client.get(effects_url.clone())
                            .send()
                            .await
                            .map_err(crate::error::Error::Reqwest)?
                            .json::<Vec<String>>()
                            .await
                            .inspect(|response| tracing::debug!(?response, "Successfully asked for effects"))
                            .inspect_err(|error| tracing::error!(?error, "WLED Client errored"))
                            .map_err(crate::error::Error::Reqwest)?;

                        let Some(effect_idx) = effects.iter().enumerate().find_map(|(idx, n)| (*n == name).then_some(idx)) else {
                            tracing::error!("{name} not found in {}", effects.join(", "));
                            continue
                        };

                        let response = wled_client
                            .post(state_url.clone())
                            .json(&wled_api_types::types::state::State {
                            seg: Some(vec![
                                wled_api_types::types::state::Seg { fx: Some(effect_idx as u16), c1, c2, c3, ..Default::default() }]),
                                ..Default::default()
                            })
                            .send()
                            .await
                            .inspect(|response| tracing::debug!(?response, "Successfully flushed state to WLED"))
                            .inspect_err(|error| tracing::error!(?error, "WLED Client errored"))
                            .map_err(crate::error::Error::Reqwest)?
                            .json::<serde_json::Value>()
                            .await
                            .map_err(crate::error::Error::Reqwest)?;

                        tracing::debug!(?response, "Received JSON response");
                        tracing::info!("Posted effect {effect_idx} successfully");

                        tokio::time::sleep(std::time::Duration::from_secs(duration_s)).await
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
