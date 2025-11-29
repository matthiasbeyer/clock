use rumqttc::v5::MqttOptions;

mod cli;
mod config;
mod error;
mod logging;
mod systemd;

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
    let mut ddp_connection = ddp_rs::connection::DDPConnection::try_new(
        format!("{}:{}", config.display.host, config.display.port),
        ddp_rs::protocol::PixelConfig::default(), // Default is RGB, 8 bits ber channel
        ddp_rs::protocol::ID::Default,
        std::net::UdpSocket::bind(format!("0.0.0.0:{}", config.display.udp_port))
            .map_err(crate::error::Error::UDPBind)?,
    )?;

    for i in 0u8..100u8 {
        let high = 10u8.overflowing_mul(i).0;

        // loop through some colors

        let temp: usize = ddp_connection.write(&[
            high, 0, 0, high, 0, 0, 0, high, 0, 0, high, 0, 0, 0, high, 0, 0, high,
        ])?;

        println!("sent {temp} packets");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // let mut every_second = tokio::time::interval(std::time::Duration::from_secs(30));
    // loop {
    //     tokio::select! {
    //         _ = every_second.tick() => {
    //         },
    //     }
    // }
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
