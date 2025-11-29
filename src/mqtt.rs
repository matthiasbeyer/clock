use rumqttc::v5::MqttOptions;

pub async fn run(
    config: crate::config::MqttConfig,
    event_sender: tokio::sync::mpsc::Sender<crate::event::Event>,
) -> Result<(), crate::error::MqttError> {
    let mut mqttoptions =
        MqttOptions::new(&config.client_name, config.host.to_string(), config.port);
    mqttoptions.set_keep_alive(config.keep_alive);

    let (mut client, mut eventloop) = rumqttc::v5::AsyncClient::new(mqttoptions, 100);

    let subscribe_topics: Vec<&'static str> = vec!["display/text", "ctrl/brightness"];

    let subs = subscribe_topics.iter().map(|topic| {
        rumqttc::v5::mqttbytes::v5::Filter::new(
            format!("{prefix}/{topic}", prefix = config.topic_prefix),
            rumqttc::v5::mqttbytes::QoS::from(config.qos),
        )
    });

    client
        .subscribe_many(subs)
        .await
        .map_err(crate::error::MqttError::Subscribing)?;

    todo!()
}
