use rumqttc::v5::MqttOptions;
use tokio_util::sync::CancellationToken;

use crate::error::MqttError;

pub async fn run(
    config: crate::config::MqttConfig,
    cancellation_token: CancellationToken,
    event_sender: tokio::sync::mpsc::Sender<crate::event::Event>,
) -> Result<(), crate::error::MqttError> {
    let mut mqttoptions =
        MqttOptions::new(&config.client_name, config.host.to_string(), config.port);
    mqttoptions.set_keep_alive(config.keep_alive);

    if let (Some(username), Some(password)) = (config.username.as_ref(), config.password.as_ref()) {
        tracing::debug!(
            ?username,
            "Setting username and password for MQTT connection"
        );
        mqttoptions.set_credentials(username, password);
    } else {
        tracing::debug!("Using no username/password for MQTT connection");
    }

    let (client, mut eventloop) = rumqttc::v5::AsyncClient::new(mqttoptions, 100);

    let topic = format!("{prefix}/events", prefix = config.topic_prefix);
    let qos = rumqttc::v5::mqttbytes::QoS::from(config.qos);

    let Some(sub_result) = cancellation_token
        .run_until_cancelled(client.subscribe(&topic, qos))
        .await
    else {
        tracing::info!("Cancelled, shutting down MQTT processing");
        return Ok(());
    };

    sub_result
        .inspect_err(|error| tracing::error!(?error, "Failed to subscribe to topic"))
        .map_err(crate::error::MqttError::Subscribing)?;

    tracing::info!("Successfully subscribed to {topic}");

    loop {
        let event = cancellation_token
            .run_until_cancelled(eventloop.poll())
            .await;

        let Some(event) = event else {
            tracing::info!("cancelled, shutting down MQTT processing");
            break;
        };

        let event = event.map_err(MqttError::Connection)?;

        match event {
            rumqttc::v5::Event::Incoming(rumqttc::v5::Incoming::Publish(
                rumqttc::v5::mqttbytes::v5::Publish {
                    dup: _,
                    qos: _,
                    retain: _,
                    topic,
                    pkid: _,
                    payload,
                    properties: _,
                },
            )) => {
                tracing::debug!(?topic, ?payload, "Received payload");

                let event: crate::event::Event = match serde_json::from_slice(&payload) {
                    Ok(event) => {
                        tracing::debug!(?event, "Deserialized event successfully");
                        event
                    }
                    Err(error) => {
                        tracing::debug!(?error, "Failed to deserialize event, ignoring");
                        continue;
                    }
                };

                if let Err(event) = event_sender.send(event).await {
                    tracing::error!(?event, "Failed to send event to internal channel");
                }
            }

            rumqttc::v5::Event::Incoming(_) => {
                // nothing
            }

            rumqttc::v5::Event::Outgoing(_outgoing) => {
                // nothing
            }
        }
    }

    Ok(())
}
