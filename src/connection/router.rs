use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::time::{Duration, Instant};
use crate::core::cemi::{Cemi, LData};
use crate::core::control_field::ControlField;
use crate::core::control_field_extended::{ExtendedControlField, AddressType};
use crate::core::layers::data::apdu::Apdu;
use crate::core::layers::data::tpdu::Tpdu;
use crate::core::layers::interfaces::apci::{Apci, ApciEnum};
use crate::core::layers::interfaces::tpci::Tpci;
use crate::core::data::knx_data_decode::DptValue;
use crate::core::data::knx_data_encode::KnxDataEncoder;
use crate::core::cache::group_address_cache::GroupAddressCache;
use crate::errors::KnxError;

const MAX_SIGNATURES_SIZE: usize = 10000;

/// Filter policies for group and individual addresses.
#[derive(Debug, Clone)]
pub struct AddressFilter {
    pub addresses: Vec<String>,
    pub policy: FilterPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterPolicy {
    DiscardAll,
    AllowAll,
}

/// Filter configuration for IP-bound or local-bound traffic.
#[derive(Debug, Clone, Default)]
pub struct DirectionFilter {
    pub group_address: Option<AddressFilter>,
    pub individual_address: Option<AddressFilter>,
}

use crate::connection::server::{KnxNetIpServer, KnxNetIpServerOptions};
use crate::connection::tunneling::{KnxTunneling, TunnelingOptions};
use crate::connection::usb::{KnxUsbConnection, KnxUsbOptions};
use crate::connection::tpuart::{TpuartConnection, TpuartOptions};

/// Configuration options for the Router.
#[derive(Debug, Clone)]
pub struct RouterOptions {
    pub individual_address: String,
    pub use_single_ia: bool,
    pub handle_hop_count: bool,
    pub to_ip_filter: DirectionFilter,
    pub to_local_filter: DirectionFilter,
    pub knx_net_ip_server: Option<KnxNetIpServerOptions>,
    pub tpuart: Option<TpuartOptions>,
    pub tunneling: Option<Vec<TunnelingOptions>>,
    pub usb: Option<KnxUsbOptions>,
}

/// A concrete trait alias/enum wrapping the known link types.
/// This avoids the need for dyn trait objects, which are not compatible with `async fn` in traits.
pub enum KnxLink {
    Tunneling(KnxTunneling),
    Usb(KnxUsbConnection),
    Tpuart(TpuartConnection),
    Server(KnxNetIpServer),
}

use super::KnxService;

impl KnxLink {
    pub async fn connect(&self) -> Result<(), KnxError> {
        match self {
            KnxLink::Tunneling(t) => t.connect().await,
            KnxLink::Usb(u) => u.connect().await,
            KnxLink::Tpuart(tp) => tp.connect().await,
            KnxLink::Server(s) => s.connect().await,
        }
    }

    pub async fn disconnect(&self) -> Result<(), KnxError> {
        match self {
            KnxLink::Tunneling(t) => t.disconnect().await,
            KnxLink::Usb(u) => u.disconnect().await,
            KnxLink::Tpuart(tp) => tp.disconnect().await,
            KnxLink::Server(s) => s.disconnect().await,
        }
    }

    pub async fn send(&self, cemi: &Cemi) -> Result<(), KnxError> {
        match self {
            KnxLink::Tunneling(t) => t.send(cemi).await,
            KnxLink::Usb(u) => u.send(cemi).await,
            KnxLink::Tpuart(tp) => tp.send(cemi).await,
            KnxLink::Server(s) => s.send(cemi).await,
        }
    }

    pub fn connection_state(&self) -> String {
        match self {
            KnxLink::Tunneling(t) => t.connection_state(),
            KnxLink::Usb(u) => u.connection_state(),
            KnxLink::Tpuart(tp) => tp.connection_state(),
            KnxLink::Server(s) => s.connection_state(),
        }
    }

    pub fn is_connected(&self) -> bool {
        match self {
            KnxLink::Tunneling(t) => t.is_connected(),
            KnxLink::Usb(u) => u.is_connected(),
            KnxLink::Tpuart(tp) => tp.is_connected(),
            KnxLink::Server(s) => s.is_connected(),
        }
    }

    pub fn individual_address(&self) -> String {
        match self {
            KnxLink::Tunneling(t) => t.individual_address(),
            KnxLink::Usb(u) => u.individual_address(),
            KnxLink::Tpuart(tp) => tp.individual_address(),
            KnxLink::Server(s) => s.individual_address(),
        }
    }
}

/// A robust, high-performance learning bridge.
///
/// Architecture based on `knxd/src/libserver/router.cpp`:
/// 1. Loop prevention through destination address tracking.
/// 2. IA learning and selective routing.
/// 3. Source address correction and sanitization.
/// 4. Address filtering between KNXnet/IP and other connections.
///
/// Port of [Router.ts](file:///f:/Proyectos/KNX.ts/src/connection/Router.ts).
pub struct Router {
    pub individual_address: String,
    pub use_single_ia: bool,
    handle_hop_count: bool,

    links: HashMap<String, KnxLink>,
    address_table: HashMap<String, String>,
    recent_destinations: HashMap<String, Instant>,

    to_ip_filter: DirectionFilter,
    to_local_filter: DirectionFilter,

    indication_tx: broadcast::Sender<(String, Cemi)>,
}

impl Router {
    pub fn new(options: RouterOptions) -> Self {
        let (indication_tx, _) = broadcast::channel(256);
        let mut router = Self {
            individual_address: options.individual_address,
            use_single_ia: options.use_single_ia,
            handle_hop_count: options.handle_hop_count,
            links: HashMap::new(),
            address_table: HashMap::new(),
            recent_destinations: HashMap::new(),
            to_ip_filter: options.to_ip_filter,
            to_local_filter: options.to_local_filter,
            indication_tx,
        };

        if let Some(mut server_opts) = options.knx_net_ip_server {
            if router.use_single_ia {
                server_opts.individual_address = router.individual_address.clone();
            }
            let key = format!("IP KNXnet/IP Server: {}:{}", server_opts.local_ip, server_opts.port);
            let server = KnxNetIpServer::new(server_opts);
            router.add_link(key, KnxLink::Server(server));
        }

        if let Some(mut tpuart_opts) = options.tpuart {
            if router.use_single_ia {
                tpuart_opts.individual_address = router.individual_address.clone();
            }
            let key = "TPUART".to_string();
            let conn = TpuartConnection::new(tpuart_opts);
            router.add_link(key, KnxLink::Tpuart(conn));
        }

        if let Some(tunneling_vec) = options.tunneling {
            for c in tunneling_vec {
                let key = format!("IP Tunneling: {}:{}", c.ip, c.port);
                let client = KnxTunneling::new(c);
                router.add_link(key, KnxLink::Tunneling(client));
            }
        }

        if let Some(mut usb_opts) = options.usb {
            if router.use_single_ia {
                usb_opts.individual_address = router.individual_address.clone();
            }
            let key = "KNXUSB".to_string();
            let conn = KnxUsbConnection::new(usb_opts);
            router.add_link(key, KnxLink::Usb(conn));
        }

        router
    }

    /// Subscribe to routed indications. Returns `(source_key, cemi)`.
    pub fn subscribe(&self) -> broadcast::Receiver<(String, Cemi)> {
        self.indication_tx.subscribe()
    }

    /// Register a link. If the key already exists, the old link is replaced.
    /// Note: Tunneling doesn't support individualAddress natively on the client side,
    /// it is assigned by the tunnel connection itself, but we track it here.
    pub fn add_link(&mut self, key: String, link: KnxLink) {
        if self.links.contains_key(&key) {
            self.address_table.retain(|_, v| v != &key);
        }
        self.links.insert(key, link);
    }

    /// Unregister a link and cleanup the routing table.
    pub fn unregister_link(&mut self, key: &str) {
        self.address_table.retain(|_, v| v != key);
        self.links.remove(key);
    }

    /// Main entry point for any packet received from any link.
    /// Based on `knxd`'s `Router::recv_L_Data` and `Router::trigger_cb` logic.
    pub async fn process_incoming(&mut self, cemi: &Cemi, source_key: &str) {
        // Process cache
        let _ = GroupAddressCache::get_instance()
            .write()
            .unwrap()
            .process_cemi(cemi);

        let (src, dest, msg_code, is_repeated, is_group, hop_count) = extract_cemi_metadata(cemi);
        let is_confirmation = msg_code == 0x2E || msg_code == 0x2F;

        // 1. Source Validation (knxd pattern)
        if !is_confirmation && !src.is_empty() && src != "0.0.0" {
            // We shouldn't receive a packet claiming to be us from the outside.
            if src == self.individual_address {
                return;
            }

            if let Some(existing_key) = self.address_table.get(&src) {
                if existing_key != source_key {
                    return; // Ignore packet from "wrong" interface
                }
            }
        }

        // 2. IA Learning
        if !is_confirmation {
            self.learn_address(&src, source_key);
        }

        // 3. Loop Prevention (knxd strict pattern)
        if is_repeated {
            if self.recent_destinations.contains_key(&dest) {
                return; // Drop repeated packet we've recently seen
            }
        }

        if self.recent_destinations.len() >= MAX_SIGNATURES_SIZE {
            if let Some(oldest_key) = self.recent_destinations.keys().next().cloned() {
                self.recent_destinations.remove(&oldest_key);
            }
        }
        self.recent_destinations.insert(dest.clone(), Instant::now());

        // 4. Route
        self.route(cemi, source_key, &dest, is_group, hop_count).await;
    }

    fn learn_address(&mut self, src: &str, source_key: &str) {
        // knxd pattern: don't learn 0.0.0 or special 15.15.255 (0xFFFF) addresses
        if src != "0.0.0" && src != "15.15.255" && !src.is_empty() {
            if self.address_table.get(src).map(|k| k.as_str()) != Some(source_key) {
                self.address_table.insert(src.to_string(), source_key.to_string());
            }
        }
    }

    async fn route(&self, cemi: &Cemi, source_key: &str, dest: &str, is_group: bool, hop_count: u8) {
        // Hop Count Management (Protect the whole network)
        if self.handle_hop_count && hop_count == 0 {
            return; // Drop packet
        }

        let is_source_ip = source_key.contains("IP");

        // Selective Routing (IA)
        if !is_group && !dest.is_empty() && dest != "0.0.0" && dest != "15.15.255" {
            if let Some(target_key) = self.address_table.get(dest) {
                if target_key != source_key {
                    if let Some(target_link) = self.links.get(target_key) {
                        let _ = target_link.send(cemi).await;
                    }
                }
                let _ = self.indication_tx.send((source_key.to_string(), cemi.clone()));
                return; // Do not flood
            }
            // If target is unknown, knxd broadcasts it to all interfaces
        }

        // Flood to all links except source, respecting filters
        for (key, link) in &self.links {
            if key == source_key {
                continue;
            }

            // Avoid looping back to the physical source address if known via another route
            if let Some(known_src_key) = self.address_table.get(dest) {
               if known_src_key == key {
                   continue;
               }
            }

            // Check if the link should filter this message
            let should_send = self.evaluate_filter(dest, is_group, is_source_ip);
            if !should_send {
                continue;
            }

            let _ = self.indication_tx.send((source_key.to_string(), cemi.clone()));
            let _ = link.send(cemi).await;
        }
    }

    fn evaluate_filter(&self, dest: &str, is_group: bool, is_source_ip: bool) -> bool {
        if is_source_ip {
            self.evaluate_direction_filter(&self.to_local_filter, dest, is_group)
        } else {
            self.evaluate_direction_filter(&self.to_ip_filter, dest, is_group)
        }
    }

    fn evaluate_direction_filter(&self, filter: &DirectionFilter, dest: &str, is_group: bool) -> bool {
        let addr_filter = if is_group {
            &filter.group_address
        } else {
            &filter.individual_address
        };

        if let Some(f) = addr_filter {
            if !f.addresses.contains(&dest.to_string()) {
                return true;
            }
            return f.policy != FilterPolicy::DiscardAll;
        }
        true
    }

    /// Periodically clean the destination cache (knxd pattern: 1-second TTL).
    pub fn gc_destinations(&mut self) {
        let now = Instant::now();
        self.recent_destinations.retain(|_, time| now.duration_since(*time) < Duration::from_secs(1));
    }

    /// Broadcast a CEMI to all registered links.
    pub async fn send_all(&self, cemi: &Cemi) -> Result<(), KnxError> {
        for (_, link) in &self.links {
            let _ = link.send(cemi).await;
        }
        Ok(())
    }

    /// Connect all registered links.
    pub async fn connect_all(&self) -> Result<(), KnxError> {
        for (_, link) in &self.links {
            link.connect().await?;
        }
        Ok(())
    }

    /// Disconnect all registered links.
    pub async fn disconnect_all(&self) {
        for (_, link) in &self.links {
            let _ = link.disconnect().await;
        }
    }

    /// Send a GroupValue_Write telegram to all links.
    pub async fn write(&self, destination: &str, dpt: &str, value: &DptValue) -> Result<(), KnxError> {
        let data = KnxDataEncoder::encode_this(dpt, value)?;
        let is_short = KnxDataEncoder::is_short_dpt(dpt);

        let cf1 = ControlField::new(0xBC);
        let cf2 = ExtendedControlField::new(0xE0);
        let tpci = Tpci::new(0x00);
        let apci = Apci::new(ApciEnum::AGroupValueWrite as u16);
        let tpdu = Tpdu {
            tpci: tpci.clone(),
            apdu: Apdu {
                tpci: tpci.clone(),
                apci,
                data: data.clone(),
                is_short,
            },
            data,
        };

        let cemi = Cemi::LDataReq(LData {
            additional_info: Vec::new(),
            control_field1: cf1,
            control_field2: cf2,
            source_address: "0.0.0".to_string(),
            destination_address: destination.to_string(),
            tpdu,
        });

        self.send_all(&cemi).await
    }

    /// Send a GroupValue_Read telegram to all links.
    pub async fn read(&self, destination: &str) -> Result<(), KnxError> {
        let cf1 = ControlField::new(0xBC);
        let cf2 = ExtendedControlField::new(0xE0);
        let tpci = Tpci::new(0x00);
        let apci = Apci::new(ApciEnum::AGroupValueRead as u16);
        let tpdu = Tpdu {
            tpci: tpci.clone(),
            apdu: Apdu {
                tpci: tpci.clone(),
                apci,
                data: vec![0],
                is_short: true,
            },
            data: vec![0],
        };

        let cemi = Cemi::LDataReq(LData {
            additional_info: Vec::new(),
            control_field1: cf1,
            control_field2: cf2,
            source_address: "0.0.0".to_string(),
            destination_address: destination.to_string(),
            tpdu,
        });

        self.send_all(&cemi).await
    }

    /// Returns a reference to the address table.
    pub fn get_address_table(&self) -> &HashMap<String, String> {
        &self.address_table
    }

    /// Returns the number of registered links.
    pub fn link_count(&self) -> usize {
        self.links.len()
    }
}

/// Extract metadata from a CEMI for routing decisions.
fn extract_cemi_metadata(cemi: &Cemi) -> (String, String, u8, bool, bool, u8) {
    match cemi {
        Cemi::LDataReq(ld) | Cemi::LDataCon(ld) | Cemi::LDataInd(ld) => {
            let src = ld.source_address.clone();
            let dest = ld.destination_address.clone();
            let msg_code = cemi.get_message_code();
            let is_repeated = !ld.control_field1.get_repeat(); // Active LOW: 0 = repeated
            let is_group = ld.control_field2.get_address_type() == AddressType::Group;
            let hop_count = ld.control_field2.get_hop_count();
            (src, dest, msg_code, is_repeated, is_group, hop_count)
        }
        _ => {
            (String::new(), String::new(), cemi.get_message_code(), false, false, 7)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_ia_learning() {
        let options = RouterOptions {
            individual_address: "15.15.0".to_string(),
            use_single_ia: true,
            handle_hop_count: false,
            to_ip_filter: DirectionFilter::default(),
            to_local_filter: DirectionFilter::default(),
            knx_net_ip_server: None,
            tpuart: None,
            tunneling: None,
            usb: None,
        };
        let mut bridge = Router::new(options);
        bridge.learn_address("1.1.1", "TPUART");
        bridge.learn_address("1.1.2", "IP Tunneling: 192.168.1.1:3671");

        assert_eq!(bridge.address_table.get("1.1.1"), Some(&"TPUART".to_string()));
        assert_eq!(bridge.address_table.get("1.1.2"), Some(&"IP Tunneling: 192.168.1.1:3671".to_string()));

        // Special addresses should not be learned
        bridge.learn_address("0.0.0", "TPUART");
        assert!(!bridge.address_table.contains_key("0.0.0"));
        bridge.learn_address("15.15.255", "TPUART");
        assert!(!bridge.address_table.contains_key("15.15.255"));
    }

    #[test]
    fn test_gc_destinations() {
        let options = RouterOptions {
            individual_address: "15.15.0".to_string(),
            use_single_ia: true,
            handle_hop_count: false,
            to_ip_filter: DirectionFilter::default(),
            to_local_filter: DirectionFilter::default(),
            knx_net_ip_server: None,
            tpuart: None,
            tunneling: None,
            usb: None,
        };
        let mut bridge = Router::new(options);

        bridge.recent_destinations.insert("1/1/1".to_string(), Instant::now() - Duration::from_secs(2));
        bridge.recent_destinations.insert("1/1/2".to_string(), Instant::now());

        bridge.gc_destinations();

        assert!(!bridge.recent_destinations.contains_key("1/1/1"));
        assert!(bridge.recent_destinations.contains_key("1/1/2"));
    }

    #[test]
    fn test_filter_evaluation() {
        let filter = DirectionFilter {
            group_address: Some(AddressFilter {
                addresses: vec!["1/1/1".to_string(), "1/1/2".to_string()],
                policy: FilterPolicy::DiscardAll,
            }),
            individual_address: None,
        };

        let options = RouterOptions {
            individual_address: "15.15.0".to_string(),
            use_single_ia: true,
            handle_hop_count: false,
            to_ip_filter: filter,
            to_local_filter: DirectionFilter::default(),
            knx_net_ip_server: None,
            tpuart: None,
            tunneling: None,
            usb: None,
        };
        let bridge = Router::new(options);

        assert!(!bridge.evaluate_filter("1/1/1", true, false));
        assert!(bridge.evaluate_filter("1/1/3", true, false));
        assert!(bridge.evaluate_filter("1.1.1", false, false));
    }
}
