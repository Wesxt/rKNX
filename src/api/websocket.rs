use crate::api::manager::ApiManager;
use crate::utils::logger::Logger;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

pub struct WebSocketServer {
    manager: Arc<ApiManager>,
    port: u16,
    logger: Logger,
}

impl WebSocketServer {
    pub fn new(manager: Arc<ApiManager>, port: u16) -> Self {
        Self {
            manager,
            port,
            logger: Logger::new("WebSocketServer"),
        }
    }

    pub fn start(self: Arc<Self>) {
        let port = self.port;
        let manager = Arc::clone(&self.manager);
        let logger = self.logger.clone();
        let self_cloned = Arc::clone(&self);

        tokio::spawn(async move {
            let addr = format!("0.0.0.0:{}", port);
            let listener = match TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(e) => {
                    logger.error(&format!(
                        "Failed to bind WebSocket server to {}: {}",
                        addr, e
                    ));
                    return;
                }
            };

            logger.info(&format!("WebSocket server listening on ws://{}", addr));

            while let Ok((stream, client_addr)) = listener.accept().await {
                let manager_cloned = Arc::clone(&manager);
                let logger_cloned = logger.clone();
                let self_cloned_inner = Arc::clone(&self_cloned);

                tokio::spawn(async move {
                    if let Ok(ws_stream) = accept_async(stream).await {
                        logger_cloned.info(&format!(
                            "New WebSocket client connected from {}",
                            client_addr
                        ));
                        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
                        let mut manager_events_rx = manager_cloned.subscribe_events();

                        // Loop for sending events to this client
                        let (event_sender_tx, mut event_sender_rx) =
                            tokio::sync::mpsc::channel::<Message>(100);

                        // Spawn a task to send messages to the ws client
                        let logger_sender = logger_cloned.clone();
                        tokio::spawn(async move {
                            while let Some(msg) = event_sender_rx.recv().await {
                                if let Err(e) = ws_sender.send(msg).await {
                                    logger_sender
                                        .warn(&format!("Failed to send WebSocket message: {}", e));
                                    break;
                                }
                            }
                        });

                        // Spawn a task to pipe ApiManager events to this client
                        let tx_for_events = event_sender_tx.clone();
                        tokio::spawn(async move {
                            while let Ok(event_val) = manager_events_rx.recv().await {
                                let msg_text = event_val.to_string();
                                if tx_for_events.send(Message::Text(msg_text)).await.is_err() {
                                    break;
                                }
                            }
                        });

                        // Main loop for receiving messages from this client
                        while let Some(Ok(msg)) = ws_receiver.next().await {
                            if let Message::Text(text) = msg {
                                let response = self_cloned_inner
                                    .handle_client_message(&manager_cloned, &text)
                                    .await;
                                let _ = event_sender_tx
                                    .send(Message::Text(response.to_string()))
                                    .await;
                            }
                        }

                        logger_cloned.info(&format!(
                            "WebSocket client from {} disconnected",
                            client_addr
                        ));
                    }
                });
            }
        });
    }

    async fn handle_client_message(&self, manager: &ApiManager, text: &str) -> Value {
        let req: Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(e) => {
                return json!({
                    "success": false,
                    "error": format!("Invalid JSON request: {}", e)
                });
            }
        };

        let msg_id = req.get("id").cloned();
        let action = match req.get("action").and_then(|a| a.as_str()) {
            Some(a) => a,
            None => {
                return json!({
                    "id": msg_id,
                    "success": false,
                    "error": "Missing action field"
                });
            }
        };

        let result = match action {
            "connect" => {
                let conn_type = req
                    .get("connection_type")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                let options = req.get("options").cloned().unwrap_or(json!({}));
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
                let addr = req
                    .get("group_address")
                    .and_then(|g| g.as_str())
                    .unwrap_or("");
                if addr.is_empty() {
                    Err("Missing group_address".to_string())
                } else {
                    match manager.subscribe(addr) {
                        Ok(_) => Ok(json!({ "subscribed": addr })),
                        Err(e) => Err(format!("Subscribe failed: {:?}", e)),
                    }
                }
            }
            "subscriptions" => {
                match manager.get_subscriptions_list() {
                    Ok(list) => Ok(list),
                    Err(e) => Err(format!("Get subscriptions failed: {:?}", e)),
                }
            }
            "unsubscribe" => {
                let addr = req
                    .get("group_address")
                    .and_then(|g| g.as_str())
                    .unwrap_or("");
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
                let addr = req
                    .get("group_address")
                    .and_then(|g| g.as_str())
                    .unwrap_or("");
                let dpt = req.get("dpt").and_then(|d| d.as_str()).unwrap_or("");
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
                    let addr = req
                        .get("group_address")
                        .and_then(|g| g.as_str())
                        .unwrap_or("");
                    let value = req.get("value").cloned().unwrap_or(Value::Null);
                    if addr.is_empty() || value.is_null() {
                        Err("Missing group_address or value".to_string())
                    } else {
                        if let Some(dpt) = req.get("dpt").and_then(|d| d.as_str()) {
                            let _ = manager.set_dpt(addr, dpt);
                        }
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
                    let addr = req
                        .get("group_address")
                        .and_then(|g| g.as_str())
                        .unwrap_or("");
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
            "get_history" => {
                let limit = req.get("limit").and_then(|l| l.as_u64()).unwrap_or(50) as usize;
                match manager.get_history(limit) {
                    Ok(h) => Ok(json!(h)),
                    Err(e) => Err(format!("Failed to retrieve history: {}", e)),
                }
            }
            "set_retention" => {
                let seconds = req
                    .get("seconds")
                    .and_then(|s| s.as_i64())
                    .unwrap_or(604800);
                match manager.set_retention(seconds) {
                    Ok(_) => Ok(json!({ "retention_configured": seconds })),
                    Err(e) => Err(format!("Failed to configure retention: {}", e)),
                }
            }
            "status" => {
                let conn_state = manager.get_connection_info();
                Ok(json!({
                    "connected": manager.is_connected(),
                    "connection_type": conn_state.as_ref().map(|c| &c.0),
                    "individual_address": conn_state.as_ref().map(|c| &c.1),
                    "subscriptions": manager.get_subscriptions(),
                    "retention_seconds": manager.get_retention().unwrap_or(604800)
                }))
            }
            _ => Err(format!("Unknown action: {}", action)),
        };

        match result {
            Ok(res) => json!({
                "id": msg_id,
                "success": true,
                "response": res
            }),
            Err(e) => json!({
                "id": msg_id,
                "success": false,
                "error": e
            }),
        }
    }
}
