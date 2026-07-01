use serde::Deserialize;
use std::fs;
use crate::connection::server::KnxNetIpServerOptions;
use crate::connection::tunneling::{TunnelingOptions, TransportProtocol};
use crate::core::knxnetip_enum::ConnectionType;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub local_ip: Option<String>,
    pub individual_address: Option<String>,
    pub friendly_name: Option<String>,
    pub mac_address: Option<String>,
    pub routing_delay: Option<u16>,
    pub client_addrs: Option<String>,
    pub use_all_interfaces: Option<bool>,
    pub is_routing: Option<bool>,
    pub max_pending_requests_per_client: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClientConfig {
    pub gateway_host: Option<String>, // maps to ip
    pub gateway_port: Option<u16>, // maps to port
    pub local_ip: Option<String>,
    pub local_port: Option<u16>,
    pub transport: Option<String>,
    pub connection_type: Option<String>,
    pub use_route_back: Option<bool>,
    pub max_queue_size: Option<usize>,
    pub auto_reconnect: Option<bool>,
    pub max_reconnect_attempts: Option<usize>,
    pub reconnect_delay_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: Option<String>,
    pub log_to_file: Option<bool>,
    pub log_dir: Option<String>,
    pub log_filename: Option<String>,
    pub indications: Option<bool>,
    pub indications_raw: Option<bool>,
    pub node_format: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AddressFilterConfig {
    pub addresses: Vec<String>,
    pub policy: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DirectionFilterConfig {
    pub group_address: Option<AddressFilterConfig>,
    pub individual_address: Option<AddressFilterConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TpuartConfig {
    pub path: String,
    pub ack_group: Option<bool>,
    pub ack_individual: Option<bool>,
    pub individual_address: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UsbConfig {
    pub path: Option<String>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub individual_address: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RouterConfig {
    pub individual_address: Option<String>,
    pub use_single_ia: Option<bool>,
    pub handle_hop_count: Option<bool>,
    pub to_ip_filter: Option<DirectionFilterConfig>,
    pub to_local_filter: Option<DirectionFilterConfig>,
    pub server: Option<ServerConfig>,
    pub tpuart: Option<TpuartConfig>,
    pub tunneling: Option<Vec<ClientConfig>>,
    pub usb: Option<UsbConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: Option<ServerConfig>,
    pub client: Option<ClientConfig>,
    pub router: Option<RouterConfig>,
    pub logging: Option<LoggingConfig>,
}

use crate::connection::router::{RouterOptions, DirectionFilter, AddressFilter, FilterPolicy};
use crate::connection::usb::KnxUsbOptions;
use crate::connection::tpuart::TpuartOptions;

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn to_server_options(&self) -> Option<KnxNetIpServerOptions> {
        let sc = self.server.as_ref()?;
        Some(KnxNetIpServerOptions {
            ip: sc.ip.clone().unwrap_or_else(|| "224.0.23.12".to_string()),
            port: sc.port.unwrap_or(3671),
            local_ip: sc.local_ip.clone().unwrap_or_else(|| "0.0.0.0".to_string()),
            individual_address: sc.individual_address.clone().unwrap_or_else(|| "1.1.0".to_string()),
            friendly_name: sc.friendly_name.clone().unwrap_or_else(|| "rKNX Server".to_string()),
            mac_address: sc.mac_address.clone().unwrap_or_else(|| "00:11:22:33:44:55".to_string()),
            routing_delay: sc.routing_delay.unwrap_or(10),
            client_addrs: sc.client_addrs.clone(),
            serial_number: None,
            use_all_interfaces: sc.use_all_interfaces.unwrap_or(false),
            is_routing: sc.is_routing.unwrap_or(true),
            max_pending_requests_per_client: sc.max_pending_requests_per_client.unwrap_or(100),
        })
    }

    pub fn to_tunneling_options(&self) -> Option<TunnelingOptions> {
        let cc = self.client.as_ref()?;
        let transport = match cc.transport.as_deref() {
            Some("Tcp") | Some("tcp") | Some("TCP") => TransportProtocol::Tcp,
            _ => TransportProtocol::Udp,
        };
        let connection_type = match cc.connection_type.as_deref() {
            Some("DeviceMgmtConnection") => ConnectionType::DeviceMgmtConnection,
            _ => ConnectionType::TunnelConnection,
        };

        Some(TunnelingOptions {
            ip: cc.gateway_host.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
            port: cc.gateway_port.unwrap_or(3671),
            local_ip: cc.local_ip.clone(),
            local_port: cc.local_port.unwrap_or(0),
            transport,
            connection_type,
            use_route_back: cc.use_route_back.unwrap_or(false),
            max_queue_size: cc.max_queue_size.unwrap_or(100),
            auto_reconnect: cc.auto_reconnect.unwrap_or(true),
            max_reconnect_attempts: cc.max_reconnect_attempts.unwrap_or(10),
            reconnect_delay_ms: cc.reconnect_delay_ms.unwrap_or(5000),
        })
    }

    pub fn to_router_options(&self) -> Option<RouterOptions> {
        let rc = self.router.as_ref()?;
        
        let map_filter = |dfc: &DirectionFilterConfig| {
            let map_addr = |afc: &AddressFilterConfig| {
                let policy = match afc.policy.as_str() {
                    "DiscardAll" | "discard all" | "Discard" => FilterPolicy::DiscardAll,
                    _ => FilterPolicy::AllowAll,
                };
                AddressFilter {
                    addresses: afc.addresses.clone(),
                    policy,
                }
            };
            DirectionFilter {
                group_address: dfc.group_address.as_ref().map(map_addr),
                individual_address: dfc.individual_address.as_ref().map(map_addr),
            }
        };

        let to_ip_filter = rc.to_ip_filter.as_ref().map(map_filter).unwrap_or_default();
        let to_local_filter = rc.to_local_filter.as_ref().map(map_filter).unwrap_or_default();

        let knx_net_ip_server = rc.server.as_ref().map(|sc| KnxNetIpServerOptions {
            ip: sc.ip.clone().unwrap_or_else(|| "224.0.23.12".to_string()),
            port: sc.port.unwrap_or(3671),
            local_ip: sc.local_ip.clone().unwrap_or_else(|| "0.0.0.0".to_string()),
            individual_address: sc.individual_address.clone().unwrap_or_else(|| "1.1.0".to_string()),
            friendly_name: sc.friendly_name.clone().unwrap_or_else(|| "rKNX Server".to_string()),
            mac_address: sc.mac_address.clone().unwrap_or_else(|| "00:11:22:33:44:55".to_string()),
            routing_delay: sc.routing_delay.unwrap_or(10),
            client_addrs: sc.client_addrs.clone(),
            serial_number: None,
            use_all_interfaces: sc.use_all_interfaces.unwrap_or(false),
            is_routing: sc.is_routing.unwrap_or(true),
            max_pending_requests_per_client: sc.max_pending_requests_per_client.unwrap_or(100),
        });

        let tpuart = rc.tpuart.as_ref().map(|tc| TpuartOptions {
            path: tc.path.clone(),
            ack_group: tc.ack_group.unwrap_or(false),
            ack_individual: tc.ack_individual.unwrap_or(false),
            individual_address: tc.individual_address.clone().unwrap_or_else(|| "1.1.0".to_string()),
        });

        let tunneling = rc.tunneling.as_ref().map(|tc_list| {
            tc_list.iter().map(|cc| {
                let transport = match cc.transport.as_deref() {
                    Some("Tcp") | Some("tcp") | Some("TCP") => TransportProtocol::Tcp,
                    _ => TransportProtocol::Udp,
                };
                let connection_type = match cc.connection_type.as_deref() {
                    Some("DeviceMgmtConnection") => ConnectionType::DeviceMgmtConnection,
                    _ => ConnectionType::TunnelConnection,
                };
                TunnelingOptions {
                    ip: cc.gateway_host.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
                    port: cc.gateway_port.unwrap_or(3671),
                    local_ip: cc.local_ip.clone(),
                    local_port: cc.local_port.unwrap_or(0),
                    transport,
                    connection_type,
                    use_route_back: cc.use_route_back.unwrap_or(false),
                    max_queue_size: cc.max_queue_size.unwrap_or(100),
                    auto_reconnect: cc.auto_reconnect.unwrap_or(true),
                    max_reconnect_attempts: cc.max_reconnect_attempts.unwrap_or(10),
                    reconnect_delay_ms: cc.reconnect_delay_ms.unwrap_or(5000),
                }
            }).collect()
        });

        let usb = rc.usb.as_ref().map(|uc| KnxUsbOptions {
            path: uc.path.clone(),
            vendor_id: uc.vendor_id,
            product_id: uc.product_id,
            individual_address: uc.individual_address.clone().unwrap_or_else(|| "1.1.0".to_string()),
        });

        Some(RouterOptions {
            individual_address: rc.individual_address.clone().unwrap_or_else(|| "15.15.0".to_string()),
            use_single_ia: rc.use_single_ia.unwrap_or(true),
            handle_hop_count: rc.handle_hop_count.unwrap_or(false),
            to_ip_filter,
            to_local_filter,
            knx_net_ip_server,
            tpuart,
            tunneling,
            usb,
        })
    }
}
