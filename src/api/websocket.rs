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

                        // 1. Send "connected"
                        let connected_payload = serde_json::json!({
                            "action": "connected",
                            "message": "KNX WebSocket Gateway connected (powered by rKNX in Rust)"
                        });
                        let _ = event_sender_tx
                            .send(Message::Text(connected_payload.to_string()))
                            .await;

                        // 2. Send "knx_connection_status"
                        let conn_info = manager_cloned.get_connection_info();
                        let status_payload = serde_json::json!({
                            "action": "knx_connection_status",
                            "connected": conn_info.is_some(),
                            "type": conn_info.as_ref().map(|c| &c.0).unwrap_or(&"none".to_string()),
                            "options": conn_info.as_ref().map(|c| &c.2).unwrap_or(&serde_json::Value::Null)
                        });
                        let _ = event_sender_tx
                            .send(Message::Text(status_payload.to_string()))
                            .await;

                        // 3. Send "subscriptions_list"
                        if let Ok(subs_list) = manager_cloned.get_subscriptions_list() {
                            let _ = event_sender_tx
                                .send(Message::Text(subs_list.to_string()))
                                .await;
                        }

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
                    "action": "error",
                    "message": format!("Invalid JSON request: {}", e)
                });
            }
        };

        let msg_id = req.get("id").cloned();
        let action = match req.get("action").and_then(|a| a.as_str()) {
            Some(a) => a,
            None => {
                return json!({
                    "action": "error",
                    "message": "Missing action field"
                });
            }
        };

        let result = match action {
            "connect_knx" => {
                let conn_type_raw = req
                    .get("connectionType")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                let conn_type = match conn_type_raw.to_lowercase().as_str() {
                    "tunneling" => "Tunneling".to_string(),
                    "router" => "Router".to_string(),
                    "knxnetipserver" => "Server".to_string(),
                    "tpuart" => "Tpuart".to_string(),
                    "usb" => "Usb".to_string(),
                    other => other.to_string(),
                };

                let raw_opts = req.get("connectionOptions").cloned().unwrap_or(json!({}));
                let mut mapped_opts = json!({});
                if let Some(obj) = raw_opts.as_object() {
                    let mut map = serde_json::Map::new();
                    for (k, v) in obj {
                        // Strip fields that aren't understood by the Rust config deserializer
                        if k == "logOptions" {
                            continue;
                        }
                        let snake_key = match k.as_str() {
                            // gateway_host / gateway_port aliases
                            "ip" => "gateway_host",
                            "host" => "gateway_host",
                            "gatewayHost" => "gateway_host",
                            "port" => "gateway_port",
                            "gatewayPort" => "gateway_port",
                            // standard camelCase -> snake_case
                            "localIp" => "local_ip",
                            "localPort" => "local_port",
                            "useRouteBack" => "use_route_back",
                            "maxQueueSize" => "max_queue_size",
                            "autoReconnect" => "auto_reconnect",
                            "maxReconnectAttempts" => "max_reconnect_attempts",
                            "reconnectDelayMs" => "reconnect_delay_ms",
                            "individualAddress" => "individual_address",
                            "friendlyName" => "friendly_name",
                            "macAddress" => "mac_address",
                            "clientAddrs" => "client_addrs",
                            "routingDelay" => "routing_delay",
                            "ackGroup" => "ack_group",
                            "ackIndividual" => "ack_individual",
                            "vendorId" => "vendor_id",
                            "productId" => "product_id",
                            other => other,
                        };

                        // Map numeric connectionType to string for ClientConfig
                        if snake_key == "connectionType" || k == "connectionType" {
                            let ct_str = match v.as_u64() {
                                Some(3) => Value::String("DeviceMgmtConnection".to_string()),
                                Some(4) | _ => Value::String("TunnelConnection".to_string()),
                            };
                            map.insert("connection_type".to_string(), ct_str);
                            continue;
                        }

                        map.insert(snake_key.to_string(), v.clone());
                    }
                    mapped_opts = Value::Object(map);
                }

                match manager.connect(&conn_type, mapped_opts).await {
                    Ok(_) => Ok(json!({ "action": "connect_knx_ack", "success": true })),
                    Err(e) => Ok(
                        json!({ "action": "connect_knx_ack", "success": false, "error": format!("{:?}", e) }),
                    ),
                }
            }
            "disconnect_knx" => match manager.disconnect().await {
                Ok(_) => Ok(json!({ "action": "disconnect_knx_ack", "success": true })),
                Err(e) => Ok(
                    json!({ "action": "disconnect_knx_ack", "success": false, "error": format!("{:?}", e) }),
                ),
            },
            "config_dpt" => {
                let addr = req
                    .get("groupAddress")
                    .and_then(|g| g.as_str())
                    .unwrap_or("");
                let dpt = req.get("dpt").and_then(|d| d.as_str()).unwrap_or("");
                if addr.is_empty() || dpt.is_empty() {
                    Err("Missing groupAddress or dpt".to_string())
                } else {
                    match manager.set_dpt(addr, dpt) {
                        Ok(_) => Ok(
                            json!({ "action": "config_dpt_ack", "groupAddress": addr, "dpt": dpt }),
                        ),
                        Err(e) => Err(format!("Set DPT failed: {:?}", e)),
                    }
                }
            }
            "subscribe" => {
                let addr = req
                    .get("groupAddress")
                    .and_then(|g| g.as_str())
                    .unwrap_or("");
                let name = req.get("name").and_then(|n| n.as_str());
                if addr.is_empty() {
                    Err("Missing groupAddress".to_string())
                } else {
                    match manager.subscribe(addr, name, None) {
                        Ok(_) => Ok(json!({ "action": "subscribe_ack", "groupAddress": addr })),
                        Err(e) => Err(format!("Subscribe failed: {:?}", e)),
                    }
                }
            }
            "unsubscribe" => {
                let addr = req
                    .get("groupAddress")
                    .and_then(|g| g.as_str())
                    .unwrap_or("");
                if addr.is_empty() {
                    Err("Missing groupAddress".to_string())
                } else {
                    match manager.unsubscribe(addr) {
                        Ok(_) => Ok(json!({ "action": "unsubscribe_ack", "groupAddress": addr })),
                        Err(e) => Err(format!("Unsubscribe failed: {:?}", e)),
                    }
                }
            }
            "read" => {
                if !manager.is_connected() {
                    Err("Connection required before sending commands".to_string())
                } else {
                    let addr = req
                        .get("groupAddress")
                        .and_then(|g| g.as_str())
                        .unwrap_or("");
                    if addr.is_empty() {
                        Err("Missing groupAddress".to_string())
                    } else {
                        match manager.read(addr).await {
                            Ok(_) => Ok(json!({ "action": "read_result", "groupAddress": addr })),
                            Err(e) => Err(format!("Read command failed: {:?}", e)),
                        }
                    }
                }
            }
            "write" => {
                if !manager.is_connected() {
                    Err("Connection required before sending commands".to_string())
                } else {
                    let addr = req
                        .get("groupAddress")
                        .and_then(|g| g.as_str())
                        .unwrap_or("");
                    let value = req.get("value").cloned().unwrap_or(Value::Null);
                    let dpt_opt = req.get("dpt").and_then(|d| d.as_str());
                    if addr.is_empty() || value.is_null() {
                        Err("Missing groupAddress or value".to_string())
                    } else {
                        if let Some(dpt) = dpt_opt {
                            let _ = manager.set_dpt(addr, dpt);
                        }
                        match manager.write(addr, value).await {
                            Ok(_) => Ok(json!({ "action": "write_ack", "groupAddress": addr })),
                            Err(e) => Err(format!("Write command failed: {:?}", e)),
                        }
                    }
                }
            }
            "query" => {
                let addr = req
                    .get("groupAddress")
                    .and_then(|g| g.as_str())
                    .unwrap_or("");
                if addr.is_empty() {
                    Err("Missing groupAddress".to_string())
                } else {
                    match manager.get_history(100) {
                        Ok(hist) => {
                            let results = hist
                                .iter()
                                .map(|item| {
                                    let ga = item
                                        .get("group_address")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    let val = item.get("value").cloned().unwrap_or(Value::Null);
                                    let cemi_desc =
                                        item.get("description").cloned().unwrap_or(Value::Null);
                                    let apci_cmd = cemi_desc
                                        .get("tpdu")
                                        .and_then(|t| t.get("apdu"))
                                        .and_then(|a| a.get("apci"))
                                        .and_then(|a| a.get("command"))
                                        .and_then(|c| c.as_str())
                                        .unwrap_or("AGroupValueWrite");

                                    let formatted_time = chrono::Local::now()
                                        .naive_local()
                                        .format("%H:%M:%S")
                                        .to_string();

                                    json!({
                                        "groupAddress": ga,
                                        "decodedValue": val,
                                        "apci": apci_cmd,
                                        "cemi": cemi_desc,
                                        "timestamp": formatted_time
                                    })
                                })
                                .collect::<Vec<Value>>();
                            Ok(json!({
                                "action": "query_result",
                                "groupAddress": addr,
                                "results": results
                            }))
                        }
                        Err(e) => Err(format!("Query failed: {}", e)),
                    }
                }
            }
            "import_group_addresses" => {
                let group_addrs = req.get("groupAddresses").and_then(|a| a.as_array());
                if let Some(addrs) = group_addrs {
                    for item in addrs {
                        let address = item.get("address").and_then(|a| a.as_str()).unwrap_or("");
                        if !address.is_empty() {
                            let dpt = item.get("dpt").and_then(|d| d.as_str());
                            let name = item.get("name").and_then(|n| n.as_str());
                            let desc = item.get("description").and_then(|d| d.as_str());

                            if let Some(dpt_str) = dpt {
                                let _ = manager.set_dpt(address, dpt_str);
                            }
                            let _ = manager.subscribe(address, name, desc);
                        }
                    }
                }
                match manager.get_subscriptions_list() {
                    Ok(list) => Ok(list),
                    Err(e) => Err(format!("Import failed: {:?}", e)),
                }
            }
            "discover" => {
                let mut local_ips = Vec::new();
                if let Ok(interfaces) = if_addrs::get_if_addrs() {
                    for iface in interfaces {
                        if !iface.is_loopback() {
                            if let std::net::IpAddr::V4(ipv4_addr) = iface.ip() {
                                local_ips.push(ipv4_addr);
                            }
                        }
                    }
                }
                if local_ips.is_empty() {
                    local_ips.push(std::net::Ipv4Addr::new(0, 0, 0, 0));
                }

                let mut tasks = Vec::new();
                for ip in local_ips {
                    let ip_str = ip.to_string();
                    tasks.push(tokio::spawn(async move {
                        crate::connection::server::KnxNetIpServer::discover(
                            &ip_str,
                            "224.0.23.12",
                            3671,
                            3000,
                            true,
                        )
                        .await
                        .unwrap_or_default()
                    }));
                }

                let mut all_devices = std::collections::HashMap::new();
                for task in tasks {
                    if let Ok(devices) = task.await {
                        for dev in devices {
                            let key = format!("{}:{}", dev.ip, dev.port);
                            all_devices.insert(key, dev);
                        }
                    }
                }

                let devices_json: Vec<serde_json::Value> = all_devices
                    .into_values()
                    .map(|dev| {
                        let mac_str = format!(
                            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                            dev.mac_address[0],
                            dev.mac_address[1],
                            dev.mac_address[2],
                            dev.mac_address[3],
                            dev.mac_address[4],
                            dev.mac_address[5]
                        );
                        json!({
                            "friendlyName": dev.friendly_name,
                            "ip": dev.ip.to_string(),
                            "port": dev.port,
                            "individualAddress": dev.individual_address,
                            "macAddress": mac_str,
                        })
                    })
                    .collect();

                Ok(json!({
                    "action": "discover_result",
                    "devices": devices_json,
                }))
            }
            "status" => {
                let conn_info = manager.get_connection_info();
                Ok(json!({
                    "action": "knx_connection_status",
                    "connected": conn_info.is_some(),
                    "type": conn_info.as_ref().map(|c| &c.0).unwrap_or(&"none".to_string()),
                    "options": conn_info.as_ref().map(|c| &c.2).unwrap_or(&serde_json::Value::Null)
                }))
            }
            _ => Err(format!("Unknown action: {}", action)),
        };

        let mut response_body = match result {
            Ok(val) => val,
            Err(err_msg) => json!({
                "action": "error",
                "message": err_msg
            }),
        };

        if let Some(id) = msg_id {
            if let Some(obj) = response_body.as_object_mut() {
                obj.insert("id".to_string(), id);
            }
        }

        response_body
    }
}
