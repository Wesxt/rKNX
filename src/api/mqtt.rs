use crate::api::manager::ApiManager;
use crate::utils::logger::Logger;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde_json::{Value, json};
use std::sync::Arc;

pub struct MqttAdapter {
    manager: Arc<ApiManager>,
    host: String,
    port: u16,
    client_id: String,
    logger: Logger,
}

impl MqttAdapter {
    pub fn new(manager: Arc<ApiManager>, host: String, port: u16, client_id: String) -> Self {
        Self {
            manager,
            host,
            port,
            client_id,
            logger: Logger::new("MqttAdapter"),
        }
    }

    pub fn start(self: Arc<Self>) {
        let manager = Arc::clone(&self.manager);
        let logger = self.logger.clone();
        let host = self.host.clone();
        let port = self.port;
        let client_id = self.client_id.clone();
        let self_cloned = Arc::clone(&self);

        tokio::spawn(async move {
            let mut mqttoptions = MqttOptions::new(&client_id, &host, port);
            mqttoptions.set_keep_alive(std::time::Duration::from_secs(5));

            let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
            logger.info(&format!(
                "MQTT client connecting to broker at {}:{}...",
                host, port
            ));

            // Subscribe to all command topics
            if let Err(e) = client.subscribe("rknx/cmd/+", QoS::AtLeastOnce).await {
                logger.error(&format!("MQTT failed to subscribe to command topic: {}", e));
                return;
            }
            logger.info("MQTT client subscribed to command topics: rknx/cmd/+");

            // Task to send indication events from manager to MQTT
            let client_sender = client.clone();
            let mut manager_rx = manager.subscribe_events();
            let logger_sender = logger.clone();
            tokio::spawn(async move {
                // Publish status online
                let _ = client_sender
                    .publish("rknx/status", QoS::AtLeastOnce, true, "online")
                    .await;

                while let Ok(event_val) = manager_rx.recv().await {
                    if let Some(addr) = event_val.get("group_address").and_then(|g| g.as_str()) {
                        let ind_topic = format!("rknx/event/indication/{}", addr);
                        let state_topic = format!("rknx/event/state/{}", addr);

                        // Publish full CEMI description
                        let payload_str = event_val.to_string();
                        if let Err(e) = client_sender
                            .publish(&ind_topic, QoS::AtLeastOnce, false, payload_str)
                            .await
                        {
                            logger_sender
                                .warn(&format!("MQTT failed to publish event indication: {}", e));
                        }

                        // Publish plain decoded state value
                        if let Some(val) = event_val.get("value") {
                            let val_str = if val.is_string() {
                                val.as_str().unwrap_or("null").to_string()
                            } else {
                                val.to_string()
                            };
                            let _ = client_sender
                                .publish(&state_topic, QoS::AtLeastOnce, false, val_str)
                                .await;
                        }
                    }
                }

                let _ = client_sender
                    .publish("rknx/status", QoS::AtLeastOnce, true, "offline")
                    .await;
            });

            // Main loop to receive commands from MQTT broker
            loop {
                match eventloop.poll().await {
                    Ok(notification) => {
                        if let Event::Incoming(Packet::Publish(publish)) = notification {
                            let topic = publish.topic;
                            let payload = match String::from_utf8(publish.payload.to_vec()) {
                                Ok(p) => p,
                                Err(_) => continue,
                            };

                            let self_inner = Arc::clone(&self_cloned);
                            let manager_inner = Arc::clone(&manager);
                            let client_inner = client.clone();

                            tokio::spawn(async move {
                                let _ = self_inner
                                    .handle_command(&manager_inner, &client_inner, &topic, &payload)
                                    .await;
                            });
                        }
                    }
                    Err(e) => {
                        logger.error(&format!("MQTT connection loop error: {}", e));
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });
    }

    async fn handle_command(
        &self,
        manager: &ApiManager,
        client: &AsyncClient,
        topic: &str,
        payload: &str,
    ) -> Result<(), &'static str> {
        let cmd = topic.strip_prefix("rknx/cmd/").unwrap_or("");
        let response_topic = format!("rknx/response/{}", cmd);

        let parsed_json: Value = serde_json::from_str(payload).unwrap_or(Value::Null);

        let result: Result<Value, String> = match cmd {
            "connect" => {
                let conn_type = parsed_json
                    .get("connection_type")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                let options = parsed_json.get("options").cloned().unwrap_or(json!({}));
                match manager.connect(conn_type, options).await {
                    Ok(_) => Ok(json!({ "connected": true })),
                    Err(e) => Err(format!("Connection failed: {:?}", e)),
                }
            }
            "disconnect" => match manager.disconnect().await {
                Ok(_) => Ok(json!({ "disconnected": true })),
                Err(e) => Err(format!("Disconnection failed: {:?}", e)),
            },
            "subscribe" => {
                let addr = if parsed_json.is_string() {
                    parsed_json.as_str().unwrap_or("")
                } else {
                    parsed_json
                        .get("group_address")
                        .and_then(|g| g.as_str())
                        .unwrap_or("")
                };

                if addr.is_empty() {
                    Err("Missing group_address".to_string())
                } else {
                    match manager.subscribe(addr) {
                        Ok(_) => Ok(json!({ "subscribed": addr })),
                        Err(e) => Err(format!("Subscribe failed: {:?}", e)),
                    }
                }
            }
            "unsubscribe" => {
                let addr = if parsed_json.is_string() {
                    parsed_json.as_str().unwrap_or("")
                } else {
                    parsed_json
                        .get("group_address")
                        .and_then(|g| g.as_str())
                        .unwrap_or("")
                };

                if addr.is_empty() {
                    Err("Missing group_address".to_string())
                } else {
                    match manager.unsubscribe(addr) {
                        Ok(_) => Ok(json!({ "unsubscribed": addr })),
                        Err(e) => Err(format!("Unsubscribe failed: {:?}", e)),
                    }
                }
            }
            "set_dpt" => {
                let addr = parsed_json
                    .get("group_address")
                    .and_then(|g| g.as_str())
                    .unwrap_or("");
                let dpt = parsed_json
                    .get("dpt")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                if addr.is_empty() || dpt.is_empty() {
                    Err("Missing group_address or dpt".to_string())
                } else {
                    match manager.set_dpt(addr, dpt) {
                        Ok(_) => Ok(json!({ "configured": addr, "dpt": dpt })),
                        Err(e) => Err(format!("Set DPT failed: {:?}", e)),
                    }
                }
            }
            "write" => {
                if !manager.is_connected() {
                    Err("Connection required before sending commands".to_string())
                } else {
                    let addr = parsed_json
                        .get("group_address")
                        .and_then(|g| g.as_str())
                        .unwrap_or("");
                    let value = parsed_json.get("value").cloned().unwrap_or(Value::Null);
                    if addr.is_empty() || value.is_null() {
                        Err("Missing group_address or value".to_string())
                    } else {
                        match manager.write(addr, value).await {
                            Ok(_) => Ok(json!({ "written": true })),
                            Err(e) => Err(format!("Write command failed: {:?}", e)),
                        }
                    }
                }
            }
            "read" => {
                if !manager.is_connected() {
                    Err("Connection required before sending commands".to_string())
                } else {
                    let addr = if parsed_json.is_string() {
                        parsed_json.as_str().unwrap_or("")
                    } else {
                        parsed_json
                            .get("group_address")
                            .and_then(|g| g.as_str())
                            .unwrap_or("")
                    };
                    if addr.is_empty() {
                        Err("Missing group_address".to_string())
                    } else {
                        match manager.read(addr).await {
                            Ok(_) => Ok(json!({ "read_sent": true })),
                            Err(e) => Err(format!("Read command failed: {:?}", e)),
                        }
                    }
                }
            }
            "set_retention" => {
                let seconds = if parsed_json.is_number() {
                    parsed_json.as_i64().unwrap_or(604800)
                } else {
                    parsed_json
                        .get("seconds")
                        .and_then(|s| s.as_i64())
                        .unwrap_or(604800)
                };
                match manager.set_retention(seconds) {
                    Ok(_) => Ok(json!({ "retention_configured": seconds })),
                    Err(e) => Err(format!("Failed to configure retention: {}", e)),
                }
            }
            _ => Err(format!("Unknown command: {}", cmd)),
        };

        let response_payload = match result {
            Ok(res) => json!({
                "success": true,
                "response": res
            }),
            Err(e) => json!({
                "success": false,
                "error": e
            }),
        };

        let _ = client
            .publish(
                &response_topic,
                QoS::AtLeastOnce,
                false,
                response_payload.to_string(),
            )
            .await;
        Ok(())
    }
}
