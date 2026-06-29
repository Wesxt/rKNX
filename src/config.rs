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
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: Option<ServerConfig>,
    pub client: Option<ClientConfig>,
    pub logging: Option<LoggingConfig>,
}

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
}
