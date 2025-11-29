use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use rumqttc::v5::MqttOptions;
use smart_leds_matrix::layout::Rectangular;
use smart_leds_matrix::SmartLedMatrix;

mod cli;
mod config;
mod error;
mod logging;
mod systemd;
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

    // Assuming you have a Vec<u8> buffer for your DDP LED matrix
    let writer = writer::Writer::new(ddp_connection);

    let mut matrix =
        SmartLedMatrix::<_, _, { 16 * 32 }>::new(writer, Rectangular::new(32, 16));

    matrix.set_brightness(10);

    // Create a text style
    let style = MonoTextStyle::new(
        &FONT_6X10,
        embedded_graphics::pixelcolor::Rgb888::new(100, 100, 100),
    );

    // Draw text to the buffer
    Text::new("LED", Point::new(0, 10), style).draw(&mut matrix).unwrap();
    matrix.flush()?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok(())
}

async fn start_mqtt_client(config: &crate::config::Config) -> Result<(), crate::error::MqttError> {
    let mut mqttoptions = MqttOptions::new(
        &config.mqtt.client_name,
        config.mqtt.host.to_string(),
        config.mqtt.port,
    );
    mqttoptions.set_keep_alive(config.mqtt.keep_alive);

    let (mut client, mut eventloop) = rumqttc::v5::AsyncClient::new(mqttoptions, 100);

    let subscribe_topics: Vec<&'static str> = vec!["display/text", "ctrl/brightness"];

    let subs = subscribe_topics.iter().map(|topic| {
        rumqttc::v5::mqttbytes::v5::Filter::new(
            format!("{prefix}/{topic}", prefix = config.mqtt.topic_prefix),
            rumqttc::v5::mqttbytes::QoS::from(config.mqtt.qos),
        )
    });

    client
        .subscribe_many(subs)
        .await
        .map_err(crate::error::MqttError::Subscribing)?;

    todo!()
}
