use crate::connection::KnxService;
use crate::connection::server::KnxNetIpServer;
use crate::connection::tunneling::KnxTunneling;
use crate::connection::usb::{KnxUsbConnection, KnxUsbOptions};
use crate::connection::tpuart::{TpuartConnection, TpuartOptions};
use crate::connection::router::Router;
use crate::core::cache::group_address_cache::GroupAddressCache;
use crate::core::cemi::Cemi;
use crate::core::data::knx_data_decode::{DptValue, KnxDataDecode};
use crate::errors::KnxError;
use crate::api::db::DbManager;
use crate::utils::logger::Logger;

use std::sync::{Arc, RwLock};
use std::collections::HashSet;
use std::time::UNIX_EPOCH;
use serde_json::Value;

#[derive(Clone)]
pub enum ActiveConnection {
    Router(Arc<Router>),
    Server(Arc<KnxNetIpServer>),
    Tpuart(Arc<TpuartConnection>),
    Tunneling(Arc<KnxTunneling>),
    Usb(Arc<KnxUsbConnection>),
}

impl ActiveConnection {
    pub async fn connect(&self) -> Result<(), KnxError> {
        match self {
            ActiveConnection::Router(r) => r.connect_all().await,
            ActiveConnection::Server(s) => s.connect().await,
            ActiveConnection::Tpuart(t) => t.connect().await,
            ActiveConnection::Tunneling(t) => t.connect().await,
            ActiveConnection::Usb(u) => u.connect().await,
        }
    }

    pub async fn disconnect(&self) -> Result<(), KnxError> {
        match self {
            ActiveConnection::Router(r) => {
                r.disconnect_all().await;
                Ok(())
            }
            ActiveConnection::Server(s) => s.disconnect().await,
            ActiveConnection::Tpuart(t) => t.disconnect().await,
            ActiveConnection::Tunneling(t) => t.disconnect().await,
            ActiveConnection::Usb(u) => u.disconnect().await,
        }
    }

    pub async fn send(&self, cemi: &Cemi) -> Result<(), KnxError> {
        match self {
            ActiveConnection::Router(r) => r.send_all(cemi).await,
            ActiveConnection::Server(s) => s.send(cemi).await,
            ActiveConnection::Tpuart(t) => t.send(cemi).await,
            ActiveConnection::Tunneling(t) => t.send(cemi).await,
            ActiveConnection::Usb(u) => u.send(cemi).await,
        }
    }

    pub fn is_connected(&self) -> bool {
        match self {
            ActiveConnection::Router(_) => true, // Routers manage multiple links
            ActiveConnection::Server(s) => s.is_connected(),
            ActiveConnection::Tpuart(t) => t.is_connected(),
            ActiveConnection::Tunneling(t) => t.is_connected(),
            ActiveConnection::Usb(u) => u.is_connected(),
        }
    }

    pub fn individual_address(&self) -> String {
        match self {
            ActiveConnection::Router(r) => r.individual_address.clone(),
            ActiveConnection::Server(s) => s.individual_address(),
            ActiveConnection::Tpuart(t) => t.individual_address(),
            ActiveConnection::Tunneling(t) => t.individual_address(),
            ActiveConnection::Usb(u) => u.individual_address(),
        }
    }

    pub async fn write(&self, destination: &str, dpt: &str, value: &DptValue) -> Result<(), KnxError> {
        match self {
            ActiveConnection::Router(r) => {
                // Router write wrapper
                let data = crate::core::data::knx_data_encode::KnxDataEncoder::encode_this(dpt, value)?;
                let is_short = crate::core::data::knx_data_encode::KnxDataEncoder::is_short_dpt(dpt);
                let cf1 = crate::core::control_field::ControlField::new(0xBC);
                let cf2 = crate::core::control_field_extended::ExtendedControlField::new(0xE0);
                let tpci = crate::core::layers::interfaces::tpci::Tpci::new(0x00);
                let apci = crate::core::layers::interfaces::apci::Apci::new(crate::core::layers::interfaces::apci::ApciEnum::AGroupValueWrite as u16);
                let tpdu = crate::core::layers::data::tpdu::Tpdu {
                    tpci: tpci.clone(),
                    apdu: crate::core::layers::data::apdu::Apdu {
                        tpci: tpci.clone(),
                        apci,
                        data: data.clone(),
                        is_short,
                    },
                    data,
                };
                let cemi = Cemi::LDataReq(crate::core::cemi::LData {
                    additional_info: Vec::new(),
                    control_field1: cf1,
                    control_field2: cf2,
                    source_address: r.individual_address.clone(),
                    destination_address: destination.to_string(),
                    tpdu,
                });
                r.send_all(&cemi).await
            }
            ActiveConnection::Server(s) => s.write(destination, dpt, value).await,
            ActiveConnection::Tpuart(t) => t.write(destination, dpt, value).await,
            ActiveConnection::Tunneling(t) => t.write(destination, dpt, value).await,
            ActiveConnection::Usb(u) => u.write(destination, dpt, value).await,
        }
    }

    pub async fn read(&self, destination: &str) -> Result<(), KnxError> {
        match self {
            ActiveConnection::Router(r) => {
                // Router read wrapper
                let cf1 = crate::core::control_field::ControlField::new(0xBC);
                let cf2 = crate::core::control_field_extended::ExtendedControlField::new(0xE0);
                let tpci = crate::core::layers::interfaces::tpci::Tpci::new(0x00);
                let apci = crate::core::layers::interfaces::apci::Apci::new(crate::core::layers::interfaces::apci::ApciEnum::AGroupValueRead as u16);
                let tpdu = crate::core::layers::data::tpdu::Tpdu {
                    tpci: tpci.clone(),
                    apdu: crate::core::layers::data::apdu::Apdu {
                        tpci: tpci.clone(),
                        apci,
                        data: vec![0],
                        is_short: true,
                    },
                    data: vec![0],
                };
                let cemi = Cemi::LDataReq(crate::core::cemi::LData {
                    additional_info: Vec::new(),
                    control_field1: cf1,
                    control_field2: cf2,
                    source_address: r.individual_address.clone(),
                    destination_address: destination.to_string(),
                    tpdu,
                });
                r.send_all(&cemi).await
            }
            ActiveConnection::Server(s) => s.read(destination).await,
            ActiveConnection::Tpuart(t) => t.read(destination).await,
            ActiveConnection::Tunneling(t) => t.read(destination).await,
            ActiveConnection::Usb(u) => u.read(destination).await,
        }
    }
}

pub struct ApiManager {
    db: DbManager,
    active_connection: Arc<RwLock<Option<ActiveConnection>>>,
    subscriptions: Arc<RwLock<HashSet<String>>>,
    event_broadcaster: tokio::sync::broadcast::Sender<Value>,
    logger: Logger,
}

impl ApiManager {
    pub fn new(db: DbManager) -> Arc<Self> {
        let (event_broadcaster, _) = tokio::sync::broadcast::channel(500);
        
        let manager = Arc::new(Self {
            db,
            active_connection: Arc::new(RwLock::new(None)),
            subscriptions: Arc::new(RwLock::new(HashSet::new())),
            event_broadcaster,
            logger: Logger::new("ApiManager"),
        });

        manager.restore_from_db();
        manager.start_db_and_event_worker();
        manager
    }

    pub fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<Value> {
        self.event_broadcaster.subscribe()
    }

    fn restore_from_db(self: &Arc<Self>) {
        // Enable GroupAddressCache
        {
            let mut cache = GroupAddressCache::get_instance().write().unwrap();
            cache.set_enabled(true);
        }

        // Restore DPT configurations
        if let Ok(dpts) = self.db.get_dpt_configs() {
            let mut cache = GroupAddressCache::get_instance().write().unwrap();
            for (addr, dpt) in dpts {
                cache.set_address_dpt(addr, dpt);
            }
        }

        // Restore Subscriptions
        if let Ok(subs) = self.db.get_subscriptions() {
            let mut guard = self.subscriptions.write().unwrap();
            for sub in subs {
                guard.insert(sub);
            }
        }

        // Restore connection
        if let Ok(Some((conn_type, opts_json, is_connected))) = self.db.get_connection_config() {
            if is_connected {
                let self_cloned = Arc::clone(self);
                tokio::spawn(async move {
                    if let Ok(opts) = serde_json::from_str::<Value>(&opts_json) {
                        let _ = self_cloned.connect_internal(&conn_type, opts).await;
                    }
                });
            }
        }
    }

    fn start_db_and_event_worker(self: &Arc<Self>) {
        let db = self.db.clone();
        let _logger = self.logger.clone();
        let subs = Arc::clone(&self.subscriptions);
        let broadcaster = self.event_broadcaster.clone();

        // Subscribe to GroupAddressCache updates
        let mut cache_rx = GroupAddressCache::get_instance().read().unwrap().subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(entry) = cache_rx.recv() => {
                        let timestamp = entry.timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
                        let addr = &entry.group_address;
                        let cemi_raw = entry.cemi.to_buffer();
                        let description = entry.cemi.describe(true);
                        let val_str = entry.decoded_value.as_ref().map(|v| format!("{:?}", v));

                        // 1. Non-blocking SQLite DB save
                        let db_cloned = db.clone();
                        let addr_cloned = addr.clone();
                        let desc_cloned = description.clone();
                        let val_cloned = val_str.clone();
                        tokio::task::spawn_blocking(move || {
                            let _ = db_cloned.save_indication(
                                timestamp,
                                &addr_cloned,
                                &cemi_raw,
                                &desc_cloned,
                                val_cloned.as_deref(),
                            );
                        });

                        // 2. Event broadcasting if subscribed
                        let is_subscribed = {
                            let guard = subs.read().unwrap();
                            guard.contains(addr)
                        };

                        if is_subscribed {
                            let event_payload = serde_json::json!({
                                "event": "indication",
                                "group_address": addr,
                                "timestamp": timestamp,
                                "description": description,
                                "value": val_str,
                            });
                            let _ = broadcaster.send(event_payload);
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {
                        // Periodic clean indications history
                        if let Ok(retention) = db.get_retention() {
                            let db_cloned = db.clone();
                            tokio::task::spawn_blocking(move || {
                                let _ = db_cloned.clean_old_indications(retention);
                            });
                        }
                    }
                }
            }
        });
    }

    async fn connect_internal(&self, conn_type: &str, opts: Value) -> Result<(), KnxError> {
        let connection = match conn_type {
            "Router" => {
                let r_opts: crate::config::RouterConfig = serde_json::from_value(opts).map_err(|_| KnxError::InvalidParametersForDpt)?;
                let config = crate::config::Config {
                    server: None,
                    client: None,
                    router: Some(r_opts),
                    logging: None,
                    api: None,
                };
                let router_opts = config.to_router_options().ok_or(KnxError::InvalidParametersForDpt)?;
                let router = Arc::new(Router::new(router_opts));
                ActiveConnection::Router(router)
            }
            "Server" => {
                let s_opts: crate::config::ServerConfig = serde_json::from_value(opts).map_err(|_| KnxError::InvalidParametersForDpt)?;
                let config = crate::config::Config {
                    server: Some(s_opts),
                    client: None,
                    router: None,
                    logging: None,
                    api: None,
                };
                let server_opts = config.to_server_options().ok_or(KnxError::InvalidParametersForDpt)?;
                let server = Arc::new(KnxNetIpServer::new(server_opts));
                ActiveConnection::Server(server)
            }
            "Tunneling" => {
                let c_opts: crate::config::ClientConfig = serde_json::from_value(opts).map_err(|_| KnxError::InvalidParametersForDpt)?;
                let config = crate::config::Config {
                    server: None,
                    client: Some(c_opts),
                    router: None,
                    logging: None,
                    api: None,
                };
                let tunneling_opts = config.to_tunneling_options().ok_or(KnxError::InvalidParametersForDpt)?;
                let tunneling = Arc::new(KnxTunneling::new(tunneling_opts));
                ActiveConnection::Tunneling(tunneling)
            }
            "Usb" => {
                let u_opts: crate::config::UsbConfig = serde_json::from_value(opts).map_err(|_| KnxError::InvalidParametersForDpt)?;
                let individual_address = u_opts.individual_address.clone().unwrap_or_else(|| "1.1.0".to_string());
                let usb_opts = KnxUsbOptions {
                    path: u_opts.path,
                    vendor_id: u_opts.vendor_id,
                    product_id: u_opts.product_id,
                    individual_address,
                };
                let usb = Arc::new(KnxUsbConnection::new(usb_opts));
                ActiveConnection::Usb(usb)
            }
            "Tpuart" => {
                let t_opts: crate::config::TpuartConfig = serde_json::from_value(opts).map_err(|_| KnxError::InvalidParametersForDpt)?;
                let individual_address = t_opts.individual_address.clone().unwrap_or_else(|| "1.1.0".to_string());
                let tpuart_opts = TpuartOptions {
                    path: t_opts.path,
                    ack_group: t_opts.ack_group.unwrap_or(false),
                    ack_individual: t_opts.ack_individual.unwrap_or(false),
                    individual_address,
                };
                let tpuart = Arc::new(TpuartConnection::new(tpuart_opts));
                ActiveConnection::Tpuart(tpuart)
            }
            _ => return Err(KnxError::InvalidParametersForDpt),
        };

        // Disconnect previous if any
        self.disconnect().await?;

        // Connect
        connection.connect().await?;

        // Save active
        {
            let mut guard = self.active_connection.write().unwrap();
            *guard = Some(connection);
        }

        Ok(())
    }

    pub async fn connect(&self, conn_type: &str, opts: Value) -> Result<(), KnxError> {
        self.connect_internal(conn_type, opts.clone()).await?;

        // Save connection options to SQLite for restart restoration
        let opts_json = opts.to_string();
        let _ = self.db.save_connection_config(conn_type, &opts_json, true);

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), KnxError> {
        let connection = {
            let mut guard = self.active_connection.write().unwrap();
            guard.take()
        };

        if let Some(conn) = connection {
            let _ = conn.disconnect().await;
            // Update connection state in DB
            if let Ok(Some((conn_type, opts_json, _))) = self.db.get_connection_config() {
                let _ = self.db.save_connection_config(&conn_type, &opts_json, false);
            }
        }
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        let guard = self.active_connection.read().unwrap();
        guard.as_ref().map(|c| c.is_connected()).unwrap_or(false)
    }

    pub fn get_connection_info(&self) -> Option<(String, String)> {
        let guard = self.active_connection.read().unwrap();
        guard.as_ref().map(|c| {
            let conn_type = match c {
                ActiveConnection::Router(_) => "Router",
                ActiveConnection::Server(_) => "Server",
                ActiveConnection::Tpuart(_) => "Tpuart",
                ActiveConnection::Tunneling(_) => "Tunneling",
                ActiveConnection::Usb(_) => "Usb",
            };
            (conn_type.to_string(), c.individual_address())
        })
    }

    pub fn subscribe(&self, group_address: &str) -> Result<(), KnxError> {
        let mut guard = self.subscriptions.write().unwrap();
        guard.insert(group_address.to_string());
        let _ = self.db.add_subscription(group_address);
        Ok(())
    }

    pub fn unsubscribe(&self, group_address: &str) -> Result<(), KnxError> {
        let mut guard = self.subscriptions.write().unwrap();
        guard.remove(group_address);
        let _ = self.db.remove_subscription(group_address);
        Ok(())
    }

    pub fn get_subscriptions(&self) -> Vec<String> {
        let guard = self.subscriptions.read().unwrap();
        guard.iter().cloned().collect()
    }

    pub fn set_dpt(&self, group_address: &str, dpt: &str) -> Result<(), KnxError> {
        let mut cache = GroupAddressCache::get_instance().write().unwrap();
        cache.set_address_dpt(group_address.to_string(), dpt.to_string());
        let _ = self.db.save_dpt_config(group_address, dpt);
        Ok(())
    }

    pub async fn write(&self, group_address: &str, value: Value) -> Result<(), KnxError> {
        let conn = {
            let guard = self.active_connection.read().unwrap();
            guard.clone().ok_or(KnxError::ConnectionClosed)?
        };

        let dpt = {
            let cache = GroupAddressCache::get_instance().read().unwrap();
            cache.get_address_dpt(group_address).ok_or(KnxError::DPTNotFound)?
        };

        let dpt_val = json_to_dpt_value(&dpt, &value)?;
        conn.write(group_address, &dpt, &dpt_val).await
    }

    pub async fn read(&self, group_address: &str) -> Result<(), KnxError> {
        let conn = {
            let guard = self.active_connection.read().unwrap();
            guard.clone().ok_or(KnxError::ConnectionClosed)?
        };

        conn.read(group_address).await
    }

    pub fn get_history(&self, limit: usize) -> Result<Vec<Value>, rusqlite::Error> {
        self.db.get_indications_history(limit)
    }

    pub fn set_retention(&self, seconds: i64) -> Result<(), rusqlite::Error> {
        self.db.set_retention(seconds)
    }

    pub fn get_retention(&self) -> Result<i64, rusqlite::Error> {
        self.db.get_retention()
    }
}

fn json_to_dpt_value(dpt: &str, val: &Value) -> Result<DptValue, KnxError> {
    let dpt_num = KnxDataDecode::get_dpt_number(dpt).ok_or(KnxError::DPTNotFound)?;
    let resolved = KnxDataDecode::fallback_dpt(dpt_num);
    match resolved {
        1 => {
            let b = val.as_bool().ok_or(KnxError::InvalidParametersForDpt)?;
            Ok(DptValue::Dpt1(b))
        }
        5 => {
            let u = val.as_u64().ok_or(KnxError::InvalidParametersForDpt)? as u8;
            Ok(DptValue::Dpt5(u))
        }
        9 | 14 => {
            let f = val.as_f64().ok_or(KnxError::InvalidParametersForDpt)? as f32;
            if resolved == 9 {
                Ok(DptValue::Dpt9(f))
            } else {
                Ok(DptValue::Dpt14(f))
            }
        }
        16 => {
            let s = val.as_str().ok_or(KnxError::InvalidParametersForDpt)?.to_string();
            Ok(DptValue::Dpt16(s))
        }
        _ => {
            if let Some(arr) = val.as_array() {
                let mut bytes = Vec::new();
                for item in arr {
                    bytes.push(item.as_u64().ok_or(KnxError::InvalidParametersForDpt)? as u8);
                }
                Ok(DptValue::Raw(bytes))
            } else if let Some(s) = val.as_str() {
                if dpt == "5.001" {
                    Ok(DptValue::Dpt5001(s.to_string()))
                } else if dpt == "5.002" {
                    Ok(DptValue::Dpt5002(s.to_string()))
                } else {
                    Ok(DptValue::Dpt16(s.to_string()))
                }
            } else {
                Err(KnxError::InvalidParametersForDpt)
            }
        }
    }
}
