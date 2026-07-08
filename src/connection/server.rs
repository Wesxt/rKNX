use std::collections::{HashMap, VecDeque};
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, Instant};

use super::KnxService;
use super::tunnel_connection::{RequestAction, TunnelConnection};
use crate::core::cemi::{Cemi, LBusmon, MPropWithPayload};
use crate::core::device_descriptor_type::DeviceDescriptorType0;
use crate::core::knxnetip_enum::{
    ConnectionType, HostProtocolCode, KnxNetIpErrorCodes, KnxNetIpServiceType,
};
use crate::core::knxnetip_header::KnxNetIpHeader;
use crate::core::knxnetip_structures::{
    Crd, Cri, DeviceInformationDib, Dib, ExtendedDeviceInformationDib, Hpai, IpConfigDib,
    IpCurrentConfigDib, RoutingBusy, RoutingLostMessage, StatusTunnelingSlot, SupportedServicesDib,
    TunnelSlot, TunnellingInfoDib,
};
use crate::errors::KnxError;
use crate::utils::knx_helper::KnxHelper;
use crate::utils::logger::Logger;

#[derive(Debug, Clone)]
pub enum ServerEvent {
    Connected,
    Disconnected,
    Error(String),
    Indication(Cemi),
    RawIndication(Vec<u8>),
    Send(Cemi),
    QueueOverflow,
    DisconnectedClient(u8),
    RoutingBusy(bool),
    RoutingReady,
    RoutingLostMessage(RoutingLostMessage),
    RoutingSystemBroadcast(Vec<u8>),
}

/// States for the KNXnetIPServer Finite State Machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnxServerState {
    Stopped,
    Starting,
    Running,
    Faulted,
}

/// Events triggering transitions in the KNXnetIPServer FSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnxServerEvent {
    Start,
    Stop,
    Running,
    Error,
}

/// Options configuration for KNXnet/IP Server.
#[derive(Debug, Clone)]
pub struct KnxNetIpServerOptions {
    pub ip: String,
    pub port: u16,
    pub local_ip: String,
    pub individual_address: String,
    pub friendly_name: String,
    pub mac_address: String,
    pub routing_delay: u16,
    pub client_addrs: Option<String>,
    pub serial_number: Option<Vec<u8>>,
    pub use_all_interfaces: bool,
    pub is_routing: bool,
    pub max_pending_requests_per_client: u32,
    pub ignore_acktimeout: bool,
}

/// Multicast pacing queue and flow control state.
struct MulticastPacing {
    msg_queue: VecDeque<Vec<u8>>,
    is_routing_busy: bool,
    last_sent_time: Instant,
    busy_counter: u32,
    last_busy_time: Instant,
}

/// Discovered KNXnet/IP device details.
#[derive(Debug, Clone)]
pub struct KnxDiscoveredDevice {
    pub ip: Ipv4Addr,
    pub port: u16,
    pub knx_medium: crate::core::knxnetip_enum::KnxMedium,
    pub device_status: u8,
    pub individual_address: u16,
    pub project_installation_id: u16,
    pub serial_number: [u8; 6],
    pub routing_multicast_address: Ipv4Addr,
    pub mac_address: [u8; 6],
    pub friendly_name: String,
}

/// Implements a KNXnet/IP Server (Gateway) that supports Routing and Tunneling protocols.
pub struct KnxNetIpServer {
    options: KnxNetIpServerOptions,
    state: Arc<RwLock<KnxServerState>>,
    socket_tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
    socket_rx: Arc<tokio::sync::Mutex<Option<mpsc::Receiver<(Vec<u8>, SocketAddr)>>>>,
    incoming_tx: broadcast::Sender<Cemi>,
    event_tx: broadcast::Sender<ServerEvent>,
    clients: Arc<RwLock<HashMap<u8, TunnelConnection>>>,

    server_ia_int: u16,
    max_tunnel_connections: u8,
    client_addrs_start_int: u16,
    multicast_pacing: Arc<std::sync::Mutex<MulticastPacing>>,
    pacing_notify: Arc<tokio::sync::Notify>,
    udp_socket: Arc<RwLock<Option<Arc<UdpSocket>>>>,
    logger: Logger,
    ignore_acktimeout: bool,
}

#[allow(dead_code)]
impl KnxNetIpServer {
    pub fn new(mut options: KnxNetIpServerOptions) -> Self {
        let (incoming_tx, _) = broadcast::channel(100);
        let (event_tx, _) = broadcast::channel(100);
        let (socket_tx, socket_rx) = mpsc::channel(100);

        if options.ip.is_empty() {
            options.ip = "224.0.23.12".to_string();
        }
        if options.port == 0 {
            options.port = 3671;
        }

        // Local IP default
        if options.local_ip.is_empty() {
            if let Some(ip) = get_local_ip() {
                options.local_ip = ip.to_string();
            } else {
                options.local_ip = "127.0.0.1".to_string();
            }
        }

        // Serial number default
        if options.serial_number.is_none() {
            if let Ok(mac) = parse_mac(&options.mac_address) {
                let mut serial = mac.to_vec();
                let port = options.port;
                serial[0] ^= ((port >> 8) & 0xFF) as u8;
                serial[1] ^= (port & 0xFF) as u8;
                options.serial_number = Some(serial);
            } else {
                options.serial_number = Some(vec![0, 0, 0, 0, 0, 0]);
            }
        }

        if options.friendly_name.is_empty() {
            options.friendly_name = "KNX.ts".to_string();
        }

        let server_ia_int = KnxHelper::get_address_from_string(&options.individual_address)
            .map(|buf| ((buf[0] as u16) << 8) | buf[1] as u16)
            .unwrap_or(0x0FFF);

        let mut max_tunnel_connections = 15;
        let mut client_addrs_start_int = server_ia_int + 1;

        if let Some(ref client_addrs) = options.client_addrs {
            let parts: Vec<&str> = client_addrs.split(':').collect();
            if parts.len() == 2 {
                if let Ok(start_buf) = KnxHelper::get_address_from_string(parts[0]) {
                    client_addrs_start_int = ((start_buf[0] as u16) << 8) | start_buf[1] as u16;
                }
                if let Ok(max_conn) = parts[1].parse::<u8>() {
                    max_tunnel_connections = max_conn;
                }
            }
        }

        let multicast_pacing = Arc::new(std::sync::Mutex::new(MulticastPacing {
            msg_queue: VecDeque::new(),
            is_routing_busy: false,
            last_sent_time: Instant::now() - Duration::from_secs(10),
            busy_counter: 0,
            last_busy_time: Instant::now() - Duration::from_secs(10),
        }));

        let logger = Logger::new("KNXnetIPServer");
        let serial_hex = options
            .serial_number
            .as_ref()
            .map(|s| s.iter().map(|b| format!("{:02X}", b)).collect::<String>())
            .unwrap_or_else(|| "000000000000".to_string());
        logger.info(&format!(
            "Initialized on {}:{}",
            options.local_ip, options.port
        ));
        logger.info(&format!("Serial Number: {}", serial_hex));

        let ignore_acktimeout = options.ignore_acktimeout;
        Self {
            options,
            state: Arc::new(RwLock::new(KnxServerState::Stopped)),
            socket_tx,
            socket_rx: Arc::new(tokio::sync::Mutex::new(Some(socket_rx))),
            incoming_tx,
            event_tx,
            clients: Arc::new(RwLock::new(HashMap::new())),
            server_ia_int,
            max_tunnel_connections,
            client_addrs_start_int,
            multicast_pacing,
            pacing_notify: Arc::new(tokio::sync::Notify::new()),
            udp_socket: Arc::new(RwLock::new(None)),
            logger,
            ignore_acktimeout,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Cemi> {
        self.incoming_tx.subscribe()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<ServerEvent> {
        self.event_tx.subscribe()
    }

    /// Discovers KNXnet/IP devices on the network.
    pub async fn discover(
        ip_local: &str,
        ip_multicast: &str,
        port: u16,
        timeout_ms: u64,
        use_extended: bool,
    ) -> Result<Vec<KnxDiscoveredDevice>, KnxError> {
        let local_ip = if ip_local.is_empty() {
            get_local_ip().unwrap_or(Ipv4Addr::new(127, 0, 0, 1))
        } else {
            Ipv4Addr::from_str(ip_local).map_err(|_| KnxError::InvalidParametersForDpt)?
        };

        let mcast_addr = format!("{}:{}", ip_multicast, port);
        let mcast_socket_addr: SocketAddr = mcast_addr
            .parse()
            .map_err(|_| KnxError::InvalidParametersForDpt)?;

        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|_| KnxError::InvalidParametersForDpt)?;
        let socket = Arc::new(socket);
        let _ = socket.set_broadcast(true);
        let _ = socket.set_multicast_ttl_v4(128);

        let bound_port = socket.local_addr().unwrap().port();

        let server_hpai = Hpai::new(HostProtocolCode::Ipv4Udp, local_ip, bound_port);
        let hpai_buf = server_hpai.to_buffer();

        // 1. Send SearchRequest
        let search_header = KnxNetIpHeader::new(
            KnxNetIpServiceType::SearchRequest,
            (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (hpai_buf.len() as u16),
        );
        let mut search_packet = search_header.to_buffer();
        search_packet.extend_from_slice(&hpai_buf);
        let _ = socket.send_to(&search_packet, &mcast_socket_addr).await;

        // 2. Send SearchRequestExtended if requested
        if use_extended {
            let ext_header = KnxNetIpHeader::new(
                KnxNetIpServiceType::SearchRequestExtended,
                (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (hpai_buf.len() as u16),
            );
            let mut ext_packet = ext_header.to_buffer();
            ext_packet.extend_from_slice(&hpai_buf);
            let _ = socket.send_to(&ext_packet, &mcast_socket_addr).await;
        }

        let mut discovered = HashMap::new();
        let mut buf = vec![0u8; 1024];
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                break;
            }

            let remaining = timeout - elapsed;
            let recv_res = tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await;

            if let Ok(Ok((len, addr))) = recv_res {
                let packet = &buf[..len];
                if packet.len() >= KnxNetIpHeader::HEADER_SIZE_10 as usize {
                    if let Ok(header) = KnxNetIpHeader::from_buffer(packet) {
                        if header.service_type == KnxNetIpServiceType::SearchResponse
                            || header.service_type == KnxNetIpServiceType::SearchResponseExtended
                        {
                            let body = &packet[KnxNetIpHeader::HEADER_SIZE_10 as usize..];
                            if body.len() >= 8 {
                                if let Ok(hpai) = Hpai::from_buffer(&body[..8]) {
                                    let mut offset = 8;
                                    let mut device_info = None;
                                    while offset < body.len() {
                                        let dib_len = body[offset] as usize;
                                        if dib_len == 0 {
                                            break;
                                        }
                                        if offset + dib_len > body.len() {
                                            break;
                                        }
                                        let dib_buf = &body[offset..offset + dib_len];
                                        if let Ok(Dib::DeviceInfo(info)) = Dib::from_buffer(dib_buf)
                                        {
                                            device_info = Some(info);
                                        }
                                        offset += dib_len;
                                    }

                                    if let Some(info) = device_info {
                                        let key = format!("{}:{}", hpai.ip_address, hpai.port);
                                        if !discovered.contains_key(&key) {
                                            let ip = match addr.ip() {
                                                std::net::IpAddr::V4(v4) => v4,
                                                _ => hpai.ip_address,
                                            };
                                            discovered.insert(
                                                key,
                                                KnxDiscoveredDevice {
                                                    ip,
                                                    port: hpai.port,
                                                    knx_medium: info.knx_medium,
                                                    device_status: info.device_status,
                                                    individual_address: info.individual_address,
                                                    project_installation_id: info
                                                        .project_installation_id,
                                                    serial_number: info
                                                        .friendly_name
                                                        .as_bytes()
                                                        .iter()
                                                        .copied()
                                                        .fold([0u8; 6], |mut acc, x| {
                                                            // deterministic fallback
                                                            acc[x as usize % 6] ^= x;
                                                            acc
                                                        }),
                                                    routing_multicast_address: info
                                                        .routing_multicast_address,
                                                    mac_address: info.mac_address,
                                                    friendly_name: info.friendly_name,
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if recv_res.is_err() {
                break;
            }
        }

        Ok(discovered.into_values().collect())
    }

    /// Helper validation Route Back (NAT Traversal)
    fn resolve_route_back(hpai: &mut Hpai, rinfo: SocketAddr) -> bool {
        let is_ip_zero = hpai.ip_address.is_unspecified();
        let is_port_zero = hpai.port == 0;

        if is_ip_zero && is_port_zero {
            hpai.ip_address = match rinfo.ip() {
                std::net::IpAddr::V4(v4) => v4,
                _ => return false,
            };
            hpai.port = rinfo.port();
            true
        } else if is_ip_zero || is_port_zero {
            false
        } else {
            true
        }
    }

    /// Resolve HPAI from RemoteInfo and local configs.
    fn get_hpai(&self, rinfo: Option<SocketAddr>) -> Hpai {
        let mut local_ip =
            Ipv4Addr::from_str(&self.options.local_ip).unwrap_or(Ipv4Addr::new(127, 0, 0, 1));
        if local_ip.is_unspecified() {
            if let Some(dest) = rinfo {
                if let Some(ip) = get_local_ip_routing_to(dest.ip()) {
                    local_ip = ip;
                } else if let Some(ip) = get_local_ip() {
                    local_ip = ip;
                }
            } else if let Some(ip) = get_local_ip() {
                local_ip = ip;
            }
        }
        Hpai::new(HostProtocolCode::Ipv4Udp, local_ip, self.options.port)
    }

    /// Generates Identification DIBs
    fn get_identification_dibs(
        &self,
        service_type: KnxNetIpServiceType,
        effective_local_ip: Ipv4Addr,
    ) -> Vec<Dib> {
        let individual_address =
            KnxHelper::get_address_from_string(&self.options.individual_address)
                .map(|buf| ((buf[0] as u16) << 8) | buf[1] as u16)
                .unwrap_or(0);

        let serial_number = {
            let mut sn = [0u8; 6];
            if let Some(ref s) = self.options.serial_number {
                let limit = s.len().min(6);
                sn[..limit].copy_from_slice(&s[..limit]);
            }
            sn
        };

        let mac_address = parse_mac(&self.options.mac_address).unwrap_or([0u8; 6]);
        let routing_multicast_address =
            Ipv4Addr::from_str(&self.options.ip).unwrap_or(Ipv4Addr::new(224, 0, 23, 12));

        let dev_info = DeviceInformationDib {
            knx_medium: crate::core::knxnetip_enum::KnxMedium::KnxIp,
            device_status: 0,
            individual_address,
            project_installation_id: 0,
            serial_number,
            routing_multicast_address,
            mac_address,
            friendly_name: self.options.friendly_name.clone(),
        };

        let supp_svc = SupportedServicesDib {
            services: vec![
                crate::core::knxnetip_structures::SupportedService { family: crate::core::knxnetip_enum::AllowedSupportedServiceFamilies::Core as u8, version: 1 },
                crate::core::knxnetip_structures::SupportedService { family: crate::core::knxnetip_enum::AllowedSupportedServiceFamilies::DeviceManagement as u8, version: 1 },
                crate::core::knxnetip_structures::SupportedService { family: crate::core::knxnetip_enum::AllowedSupportedServiceFamilies::Tunnelling as u8, version: 1 },
                crate::core::knxnetip_structures::SupportedService { family: crate::core::knxnetip_enum::AllowedSupportedServiceFamilies::Routing as u8, version: 1 },
            ]
        };

        if service_type == KnxNetIpServiceType::SearchResponse {
            return vec![Dib::DeviceInfo(dev_info), Dib::SupportedServices(supp_svc)];
        }

        let device_descriptor_type0 = DeviceDescriptorType0::new(
            crate::core::device_descriptor_type::DeviceDescriptorType0::KNXNET_IP_ROUTER.value(),
        );
        let ext_dev_info = ExtendedDeviceInformationDib {
            medium_status: false,
            maximal_local_apdu_length: 254,
            device_descriptor_type0,
        };

        let subnet_mask = Ipv4Addr::new(255, 255, 255, 0);

        let ip_config = IpConfigDib {
            ip_address: effective_local_ip,
            subnet_mask,
            default_gateway: Ipv4Addr::new(0, 0, 0, 0),
            ip_capabilities: 0x01,
            ip_assignment_method: 0x02,
        };

        let ip_current = IpCurrentConfigDib {
            ip_address: effective_local_ip,
            subnet_mask,
            default_gateway: Ipv4Addr::new(0, 0, 0, 0),
            dhcp_server: Ipv4Addr::new(0, 0, 0, 0),
            ip_assignment_method: 0x02,
        };

        let mut slots = Vec::new();
        let clients = self.clients.read().unwrap();
        for i in 1..=self.max_tunnel_connections {
            let conn = clients.get(&i);
            let mut status = StatusTunnelingSlot::default();
            status.set_authorised(true);
            status.set_usable(conn.is_some());
            status.set_free(conn.is_none());
            slots.push(TunnelSlot {
                address: conn
                    .map(|c| c.knx_address)
                    .unwrap_or(self.client_addrs_start_int + (i as u16) - 1),
                status,
            });
        }

        let tunnelling_info = TunnellingInfoDib {
            apdu_length: 254,
            slots,
        };

        vec![
            Dib::DeviceInfo(dev_info),
            Dib::SupportedServices(supp_svc),
            Dib::ExtendedDeviceInfo(ext_dev_info),
            Dib::IpConfig(ip_config),
            Dib::IpCurrentConfig(ip_current),
            Dib::TunnellingInfo(tunnelling_info),
        ]
    }

    /// Handles incoming UDP packets based on their Service Type.
    async fn handle_message(
        &self,
        msg: &[u8],
        rinfo: SocketAddr,
        socket: &Arc<UdpSocket>,
    ) -> Result<(), KnxError> {
        if msg.len() < KnxNetIpHeader::HEADER_SIZE_10.into() {
            return Err(KnxError::InvalidParametersForDpt);
        }

        let header = KnxNetIpHeader::from_buffer(msg)?;
        let body = &msg[KnxNetIpHeader::HEADER_SIZE_10 as usize..];

        // Anti-Echo Check by IP/Port
        let local_ip =
            Ipv4Addr::from_str(&self.options.local_ip).unwrap_or(Ipv4Addr::new(127, 0, 0, 1));
        if let SocketAddr::V4(addr_v4) = rinfo {
            if *addr_v4.ip() == local_ip && addr_v4.port() == self.options.port {
                return Ok(());
            }
        }

        match header.service_type {
            KnxNetIpServiceType::SearchRequest | KnxNetIpServiceType::SearchRequestExtended => {
                let is_extended = header.service_type == KnxNetIpServiceType::SearchRequestExtended;
                let mut client_hpai = Hpai::from_buffer(body)?;
                if !Self::resolve_route_back(&mut client_hpai, rinfo) {
                    return Ok(());
                }

                let server_hpai = self.get_hpai(Some(rinfo));
                let response_type = if is_extended {
                    KnxNetIpServiceType::SearchResponseExtended
                } else {
                    KnxNetIpServiceType::SearchResponse
                };

                let dibs = self.get_identification_dibs(response_type, server_hpai.ip_address);

                let mut res_body = server_hpai.to_buffer();
                for dib in dibs {
                    res_body.extend_from_slice(&dib.to_buffer());
                }

                let res_header = KnxNetIpHeader::new(
                    response_type,
                    (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
                );

                let mut packet = res_header.to_buffer();
                packet.extend_from_slice(&res_body);

                let dest = SocketAddr::new(
                    std::net::IpAddr::V4(client_hpai.ip_address),
                    client_hpai.port,
                );
                let _ = socket.send_to(&packet[..], &dest).await;
            }

            KnxNetIpServiceType::DescriptionRequest => {
                let mut client_hpai = Hpai::from_buffer(body)?;
                if !Self::resolve_route_back(&mut client_hpai, rinfo) {
                    return Ok(());
                }

                let server_hpai = self.get_hpai(Some(rinfo));
                let dibs = self.get_identification_dibs(
                    KnxNetIpServiceType::DescriptionResponse,
                    server_hpai.ip_address,
                );

                let mut res_body = Vec::new();
                for dib in dibs {
                    res_body.extend_from_slice(&dib.to_buffer());
                }

                let res_header = KnxNetIpHeader::new(
                    KnxNetIpServiceType::DescriptionResponse,
                    (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
                );

                let mut packet = res_header.to_buffer();
                packet.extend_from_slice(&res_body);

                let dest = SocketAddr::new(
                    std::net::IpAddr::V4(client_hpai.ip_address),
                    client_hpai.port,
                );
                let _ = socket.send_to(&packet[..], &dest).await;
            }

            KnxNetIpServiceType::ConnectRequest => {
                let mut client_control_hpai = Hpai::from_buffer(&body[..8])?;
                let mut client_data_hpai = Hpai::from_buffer(&body[8..16])?;

                if !Self::resolve_route_back(&mut client_control_hpai, rinfo) {
                    return Ok(());
                }
                if !Self::resolve_route_back(&mut client_data_hpai, rinfo) {
                    return Ok(());
                }

                let cri = Cri::from_buffer(&body[16..])?;
                let mut status = KnxNetIpErrorCodes::ENoError as u8;
                let mut channel_id = 0;

                let server_data_hpai = self.get_hpai(Some(rinfo));

                let mut clients = self.clients.write().unwrap();

                // Replace stale connections
                if let Some(addr) = cri.individual_address {
                    if addr != 0 {
                        let mut stale_channel = None;
                        for (&cid, conn) in clients.iter() {
                            if conn.knx_address == addr
                                && conn.control_hpai.ip_address == client_control_hpai.ip_address
                            {
                                stale_channel = Some(cid);
                                break;
                            }
                        }
                        if let Some(cid) = stale_channel {
                            if let Some(mut conn) = clients.remove(&cid) {
                                conn.close();
                                let _ = self.event_tx.send(ServerEvent::DisconnectedClient(cid));
                            }
                        }
                    }
                }

                for i in 1..=self.max_tunnel_connections {
                    if !clients.contains_key(&i) {
                        channel_id = i;
                        break;
                    }
                }

                if channel_id == 0 {
                    status = KnxNetIpErrorCodes::ENoMoreConnections as u8;
                } else if cri.connection_type == ConnectionType::DeviceMgmtConnection {
                    let conn = TunnelConnection::new(
                        channel_id,
                        client_control_hpai.clone(),
                        client_data_hpai.clone(),
                        0,
                        "0.0.0".to_string(),
                        crate::core::knxnetip_enum::KnxLayer::from_u8(cri.knx_layer)
                            .unwrap_or(crate::core::knxnetip_enum::KnxLayer::LinkLayer),
                        120_000,
                        1_000,
                        100,
                        self.ignore_acktimeout,
                    );
                    clients.insert(channel_id, conn);

                    let mut res_body = vec![channel_id, status];
                    res_body.extend_from_slice(&server_data_hpai.to_buffer());
                    res_body.extend_from_slice(&[0x02, ConnectionType::DeviceMgmtConnection as u8]);

                    let res_header = KnxNetIpHeader::new(
                        KnxNetIpServiceType::ConnectResponse,
                        (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
                    );
                    let mut packet = res_header.to_buffer();
                    packet.extend_from_slice(&res_body);

                    let dest = SocketAddr::new(
                        std::net::IpAddr::V4(client_control_hpai.ip_address),
                        client_control_hpai.port,
                    );
                    let _ = socket.send_to(&packet[..], &dest).await;
                    return Ok(());
                } else if cri.connection_type == ConnectionType::TunnelConnection {
                    let mut knx_addr = cri.individual_address.unwrap_or(0);
                    if knx_addr == 0 {
                        knx_addr = self.client_addrs_start_int + (channel_id as u16) - 1;
                    }

                    if status == KnxNetIpErrorCodes::ENoError as u8 {
                        for (_, conn) in clients.iter() {
                            if conn.knx_address == knx_addr {
                                status = KnxNetIpErrorCodes::ENoMoreUniqueConnections as u8;
                                break;
                            }
                        }
                    }

                    if status == KnxNetIpErrorCodes::ENoError as u8 {
                        let knx_address_str =
                            KnxHelper::get_address_from_number(knx_addr, ".", false)
                                .unwrap_or_else(|_| "0.0.0".to_string());

                        let conn = TunnelConnection::new(
                            channel_id,
                            client_control_hpai.clone(),
                            client_data_hpai.clone(),
                            knx_addr,
                            knx_address_str,
                            crate::core::knxnetip_enum::KnxLayer::from_u8(cri.knx_layer)
                                .unwrap_or(crate::core::knxnetip_enum::KnxLayer::LinkLayer),
                            120_000,
                            1_000,
                            100,
                            self.ignore_acktimeout,
                        );
                        clients.insert(channel_id, conn);

                        let crd = Crd::new(ConnectionType::TunnelConnection, knx_addr);
                        let mut res_body = vec![channel_id, status];
                        res_body.extend_from_slice(&server_data_hpai.to_buffer());
                        res_body.extend_from_slice(&crd.to_buffer());

                        let res_header = KnxNetIpHeader::new(
                            KnxNetIpServiceType::ConnectResponse,
                            (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
                        );
                        let mut packet = res_header.to_buffer();
                        packet.extend_from_slice(&res_body);

                        let dest = SocketAddr::new(
                            std::net::IpAddr::V4(client_control_hpai.ip_address),
                            client_control_hpai.port,
                        );
                        let _ = socket.send_to(&packet[..], &dest).await;
                        return Ok(());
                    }
                } else {
                    status = KnxNetIpErrorCodes::EConnectionType as u8;
                }

                // Failed connection
                let res_header = KnxNetIpHeader::new(KnxNetIpServiceType::ConnectResponse, 8);
                let mut packet = res_header.to_buffer();
                packet.extend_from_slice(&[0, status]);
                let dest = SocketAddr::new(
                    std::net::IpAddr::V4(client_control_hpai.ip_address),
                    client_control_hpai.port,
                );
                let _ = socket.send_to(&packet[..], &dest).await;
            }

            KnxNetIpServiceType::ConnectionStateRequest => {
                let channel_id = body[0];
                let mut client_control_hpai = Hpai::from_buffer(&body[2..])?;
                if !Self::resolve_route_back(&mut client_control_hpai, rinfo) {
                    return Ok(());
                }

                let mut status = KnxNetIpErrorCodes::EConnectionId as u8;
                {
                    let mut clients = self.clients.write().unwrap();
                    if let Some(conn) = clients.get_mut(&channel_id) {
                        status = KnxNetIpErrorCodes::ENoError as u8;
                        conn.reset_heartbeat();
                    }
                }

                let res_body = vec![channel_id, status];
                let res_header = KnxNetIpHeader::new(
                    KnxNetIpServiceType::ConnectionStateResponse,
                    (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
                );
                let mut packet = res_header.to_buffer();
                packet.extend_from_slice(&res_body);

                let dest = SocketAddr::new(
                    std::net::IpAddr::V4(client_control_hpai.ip_address),
                    client_control_hpai.port,
                );
                let _ = socket.send_to(&packet[..], &dest).await;
            }

            KnxNetIpServiceType::DisconnectRequest => {
                let channel_id = body[0];
                let mut client_control_hpai = Hpai::from_buffer(&body[2..])?;
                if !Self::resolve_route_back(&mut client_control_hpai, rinfo) {
                    return Ok(());
                }

                {
                    let mut clients = self.clients.write().unwrap();
                    if let Some(mut conn) = clients.remove(&channel_id) {
                        conn.close();
                        let _ = self
                            .event_tx
                            .send(ServerEvent::DisconnectedClient(channel_id));
                    }
                }

                let res_body = vec![channel_id, KnxNetIpErrorCodes::ENoError as u8];
                let res_header = KnxNetIpHeader::new(
                    KnxNetIpServiceType::DisconnectResponse,
                    (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
                );
                let mut packet = res_header.to_buffer();
                packet.extend_from_slice(&res_body);

                let dest = SocketAddr::new(
                    std::net::IpAddr::V4(client_control_hpai.ip_address),
                    client_control_hpai.port,
                );
                let _ = socket.send_to(&packet[..], &dest).await;
            }

            KnxNetIpServiceType::TunnellingRequest => {
                let header_len = body[0] as usize;
                let channel_id = body[1];
                let seq = body[2];
                let cemi_bytes = &body[header_len..];

                let (action, status, knx_layer, knx_address) = {
                    let mut clients = self.clients.write().unwrap();
                    if let Some(conn) = clients.get_mut(&channel_id) {
                        // Rate limiting / flooding check
                        let now = Instant::now();
                        if now.duration_since(conn.last_rx_time).as_secs() >= 1 {
                            conn.rx_count = 0;
                            conn.last_rx_time = now;
                        }
                        conn.rx_count += 1;

                        // Terminate if client is flooding
                        let flood_threshold = self.options.max_pending_requests_per_client;
                        if flood_threshold > 0 && conn.rx_count > flood_threshold {
                            conn.close();
                            clients.remove(&channel_id);
                            let _ = self
                                .event_tx
                                .send(ServerEvent::DisconnectedClient(channel_id));

                            // Send DISCONNECT_REQUEST to client
                            let server_hpai = self.get_hpai(Some(rinfo));
                            let mut dis_body = vec![channel_id, 0x00];
                            dis_body.extend_from_slice(&server_hpai.to_buffer());
                            let dis_header = KnxNetIpHeader::new(
                                KnxNetIpServiceType::DisconnectRequest,
                                (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (dis_body.len() as u16),
                            );
                            let mut packet = dis_header.to_buffer();
                            packet.extend_from_slice(&dis_body);
                            let _ = socket.send_to(&packet[..], &rinfo).await;
                            return Ok(());
                        }

                        let val = conn.validate_request(seq);
                        (val.action, val.status, conn.knx_layer, conn.knx_address)
                    } else {
                        (
                            RequestAction::RetransmitAck,
                            KnxNetIpErrorCodes::EConnectionId as u8,
                            crate::core::knxnetip_enum::KnxLayer::LinkLayer,
                            0,
                        )
                    }
                };

                if action == RequestAction::RetransmitAck {
                    self.send_tunnel_ack(channel_id, seq, status, rinfo, socket)
                        .await;
                    return Ok(());
                }
                if action == RequestAction::Discard {
                    return Ok(());
                }

                self.send_tunnel_ack(channel_id, seq, status, rinfo, socket)
                    .await;

                let msg_code = cemi_bytes[0];
                let add_info_len = cemi_bytes[1];

                if knx_layer == crate::core::knxnetip_enum::KnxLayer::BusmonitorLayer
                    && (msg_code == 0x11 || msg_code == 0x10)
                {
                    return Ok(());
                }

                let mut mut_cemi_bytes = cemi_bytes.to_vec();
                if msg_code == 0x11 {
                    let src_ia_offset = 2 + add_info_len as usize + 2;
                    if src_ia_offset + 1 < mut_cemi_bytes.len() {
                        let src_ia = ((mut_cemi_bytes[src_ia_offset] as u16) << 8)
                            | (mut_cemi_bytes[src_ia_offset + 1] as u16);
                        if src_ia == 0 {
                            mut_cemi_bytes[src_ia_offset] = (knx_address >> 8) as u8;
                            mut_cemi_bytes[src_ia_offset + 1] = (knx_address & 0xFF) as u8;
                        }
                    }
                }

                if let Ok(cemi) = Cemi::from_buffer(&mut_cemi_bytes) {
                    let _ = self.incoming_tx.send(cemi.clone());
                    let _ = self.event_tx.send(ServerEvent::Indication(cemi.clone()));
                    let _ = self
                        .event_tx
                        .send(ServerEvent::RawIndication(mut_cemi_bytes.clone()));
                    self.logger.log_indication(&cemi);
                    self.logger.log_indication_raw(&mut_cemi_bytes);
                    let _ =
                        crate::core::cache::group_address_cache::GroupAddressCache::get_instance()
                            .write()
                            .unwrap()
                            .process_cemi(&cemi);
                }

                let mut routing_cemi = mut_cemi_bytes.clone();
                if msg_code == 0x11 {
                    routing_cemi[0] = 0x29; // L_Data.ind
                } else if msg_code == 0x10 {
                    routing_cemi[0] = 0x2d; // L_Poll_Data.con
                }

                self.enqueue_packet(socket, &routing_cemi).await;

                if msg_code == 0x11 || msg_code == 0x10 {
                    let mut con_cemi = mut_cemi_bytes.clone();
                    con_cemi[0] = msg_code + 0x1D;
                    con_cemi[2 + add_info_len as usize] &= 0xFE;

                    let mut clients = self.clients.write().unwrap();
                    if let Some(conn) = clients.get_mut(&channel_id) {
                        conn.enqueue(&con_cemi, KnxNetIpServiceType::TunnellingRequest);
                        if let Some(packet) = conn.process_queue() {
                            let _ = socket.send_to(&packet[..], &conn.data_endpoint()).await;
                        }
                    }
                }
            }

            KnxNetIpServiceType::TunnellingAck => {
                let channel_id = body[1];
                let seq = body[2];
                let status = body[3];

                let mut clients = self.clients.write().unwrap();
                if let Some(conn) = clients.get_mut(&channel_id) {
                    let _ = conn.handle_ack(seq, status);

                    if let Some(packet) = conn.process_queue() {
                        let dest = conn.data_endpoint();
                        let socket_cloned = Arc::clone(socket);
                        tokio::spawn(async move {
                            let _ = socket_cloned.send_to(&packet[..], &dest).await;
                        });
                    }
                }
            }

            KnxNetIpServiceType::DeviceConfigurationRequest => {
                let header_len = body[0] as usize;
                let channel_id = body[1];
                let seq = body[2];
                let cemi_bytes = &body[header_len..];

                let (action, status) = {
                    let mut clients = self.clients.write().unwrap();
                    if let Some(conn) = clients.get_mut(&channel_id) {
                        let val = conn.validate_request(seq);
                        (val.action, val.status)
                    } else {
                        (
                            RequestAction::RetransmitAck,
                            KnxNetIpErrorCodes::EConnectionId as u8,
                        )
                    }
                };

                if action == RequestAction::RetransmitAck {
                    self.send_device_config_ack(channel_id, seq, status, socket)
                        .await;
                    return Ok(());
                }
                if action == RequestAction::Discard {
                    return Ok(());
                }

                self.send_device_config_ack(
                    channel_id,
                    seq,
                    KnxNetIpErrorCodes::ENoError as u8,
                    socket,
                )
                .await;

                if let Ok(cemi) = Cemi::from_buffer(cemi_bytes) {
                    if let Cemi::MPropReadReq(req) = cemi {
                        let mut data = Vec::new();
                        if req.interface_object_type == 0 && req.property_id == 1 {
                            if let Ok(addr_buf) =
                                KnxHelper::get_address_from_string(&self.options.individual_address)
                            {
                                data = addr_buf.to_vec();
                            }
                        }

                        let res_cemi = Cemi::MPropReadCon(MPropWithPayload {
                            interface_object_type: req.interface_object_type,
                            object_instance: req.object_instance,
                            property_id: req.property_id,
                            number_of_elements: req.number_of_elements,
                            start_index: req.start_index,
                            data,
                        });

                        let mut clients = self.clients.write().unwrap();
                        if let Some(conn) = clients.get_mut(&channel_id) {
                            conn.enqueue(
                                &res_cemi.to_buffer(),
                                KnxNetIpServiceType::DeviceConfigurationRequest,
                            );
                            if let Some(packet) = conn.process_queue() {
                                let _ = socket.send_to(&packet[..], &conn.data_endpoint()).await;
                            }
                        }
                    }
                }
            }

            KnxNetIpServiceType::DeviceConfigurationAck => {
                let channel_id = body[1];
                let seq = body[2];
                let status = body[3];

                let mut clients = self.clients.write().unwrap();
                if let Some(conn) = clients.get_mut(&channel_id) {
                    let _ = conn.handle_ack(seq, status);

                    if let Some(packet) = conn.process_queue() {
                        let dest = conn.data_endpoint();
                        let socket_cloned = Arc::clone(socket);
                        tokio::spawn(async move {
                            let _ = socket_cloned.send_to(&packet[..], &dest).await;
                        });
                    }
                }
            }

            KnxNetIpServiceType::RoutingIndication => {
                let add_info_len = body[1] as usize;
                if body.len() >= 6 + add_info_len {
                    let src_ia =
                        ((body[4 + add_info_len] as u16) << 8) | (body[5 + add_info_len] as u16);
                    if src_ia == self.server_ia_int {
                        return Ok(()); // Anti-Eco Multicast
                    }
                }

                let cemi_start = 6;
                if let Ok(cemi) = Cemi::from_buffer(&msg[cemi_start..]) {
                    let _ = self.incoming_tx.send(cemi.clone());
                    let _ = self.event_tx.send(ServerEvent::Indication(cemi.clone()));
                    let _ = self
                        .event_tx
                        .send(ServerEvent::RawIndication(msg[cemi_start..].to_vec()));
                    self.logger.log_indication(&cemi);
                    self.logger.log_indication_raw(&msg[cemi_start..]);
                    let _ =
                        crate::core::cache::group_address_cache::GroupAddressCache::get_instance()
                            .write()
                            .unwrap()
                            .process_cemi(&cemi);

                    // Forward to tunnels
                    let src_ia_str = match &msg[cemi_start..] {
                        _ => "".to_string(), // Keep it simple
                    };
                    let busmon_body = convert_data_ind_to_busmon_ind(&msg[cemi_start..]);

                    let mut clients = self.clients.write().unwrap();
                    for conn in clients.values_mut() {
                        if src_ia_str == conn.knx_address_str {
                            continue;
                        }
                        if conn.knx_layer == crate::core::knxnetip_enum::KnxLayer::BusmonitorLayer {
                            conn.enqueue(&busmon_body, KnxNetIpServiceType::TunnellingRequest);
                        } else {
                            conn.enqueue(
                                &msg[cemi_start..],
                                KnxNetIpServiceType::TunnellingRequest,
                            );
                        }

                        if let Some(packet) = conn.process_queue() {
                            let dest = conn.data_endpoint();
                            let socket_cloned = Arc::clone(socket);
                            tokio::spawn(async move {
                                let _ = socket_cloned.send_to(&packet[..], &dest).await;
                            });
                        }
                    }
                }
            }

            KnxNetIpServiceType::RoutingBusy => {
                if let Ok(busy) = RoutingBusy::from_buffer(body) {
                    self.handle_routing_busy(socket, busy);
                }
            }

            KnxNetIpServiceType::RoutingLostMessage => {
                if let Ok(lost) = RoutingLostMessage::from_buffer(body) {
                    let cemi = Cemi::LBusmonInd(LBusmon {
                        additional_info: Vec::new(),
                        data: lost.to_buffer(),
                    });
                    let _ = self.incoming_tx.send(cemi.clone());
                    let _ = self
                        .event_tx
                        .send(ServerEvent::RoutingLostMessage(lost.clone()));
                    let _ = self.event_tx.send(ServerEvent::Indication(cemi.clone()));
                    let _ = self
                        .event_tx
                        .send(ServerEvent::RawIndication(body.to_vec()));
                    self.logger.log_indication(&cemi);
                    self.logger.log_indication_raw(body);
                }
            }

            _ => {}
        }

        Ok(())
    }

    async fn send_tunnel_ack(
        &self,
        channel_id: u8,
        seq: u8,
        status: u8,
        rinfo: SocketAddr,
        socket: &Arc<UdpSocket>,
    ) {
        let body = vec![0x04, channel_id, seq, status];
        let header = KnxNetIpHeader::new(
            KnxNetIpServiceType::TunnellingAck,
            (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (body.len() as u16),
        );
        let mut packet = header.to_buffer();
        packet.extend_from_slice(&body);

        let dest = {
            let clients = self.clients.read().unwrap();
            if let Some(conn) = clients.get(&channel_id) {
                conn.data_endpoint().parse::<SocketAddr>().ok()
            } else {
                None
            }
        };

        let dest_addr = dest.unwrap_or(rinfo);
        let _ = socket.send_to(&packet[..], &dest_addr).await;
    }

    async fn send_device_config_ack(
        &self,
        channel_id: u8,
        seq: u8,
        status: u8,
        socket: &Arc<UdpSocket>,
    ) {
        let body = vec![0x04, channel_id, seq, status];
        let header = KnxNetIpHeader::new(
            KnxNetIpServiceType::DeviceConfigurationAck,
            (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (body.len() as u16),
        );
        let mut packet = header.to_buffer();
        packet.extend_from_slice(&body);

        let dest = {
            let clients = self.clients.read().unwrap();
            if let Some(conn) = clients.get(&channel_id) {
                conn.data_endpoint().parse::<SocketAddr>().ok()
            } else {
                None
            }
        };

        if let Some(dest_addr) = dest {
            let _ = socket.send_to(&packet[..], &dest_addr).await;
        }
    }

    /// Enqueues routing packet for paced multicast sending.
    async fn enqueue_packet(&self, socket: &Arc<UdpSocket>, cemi_bytes: &[u8]) {
        let header = KnxNetIpHeader::new(
            KnxNetIpServiceType::RoutingIndication,
            (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (cemi_bytes.len() as u16),
        );
        let mut packet = header.to_buffer();
        packet.extend_from_slice(cemi_bytes);

        let is_busy = {
            let mut pacing = self.multicast_pacing.lock().unwrap();
            if pacing.msg_queue.len() >= 100 {
                Self::send_lost_message(socket, &self.options, 1);
                let _ = self.event_tx.send(ServerEvent::QueueOverflow);
                return;
            }

            pacing.msg_queue.push_back(packet);

            if pacing.msg_queue.len() >= 15 && !pacing.is_routing_busy {
                let delay = self.options.routing_delay as usize;
                let wait_time = (delay * pacing.msg_queue.len()).min(100) as u16;
                Self::send_routing_busy(socket, &self.options, wait_time);
                pacing.is_routing_busy = true;
                let _ = self.event_tx.send(ServerEvent::RoutingBusy(true));

                let pacing_cloned = Arc::clone(&self.multicast_pacing);
                let notify_cloned = Arc::clone(&self.pacing_notify);
                let event_tx_cloned = self.event_tx.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(wait_time as u64)).await;
                    let mut p = pacing_cloned.lock().unwrap();
                    p.is_routing_busy = false;
                    let _ = event_tx_cloned.send(ServerEvent::RoutingBusy(false));
                    let _ = event_tx_cloned.send(ServerEvent::RoutingReady);
                    notify_cloned.notify_one();
                });
            }
            pacing.is_routing_busy
        };

        if !is_busy {
            self.pacing_notify.notify_one();
        }
    }

    fn send_lost_message(socket: &Arc<UdpSocket>, options: &KnxNetIpServerOptions, count: u16) {
        let lost_msg = RoutingLostMessage::new(0, count);
        let msg_body = lost_msg.to_buffer();
        let header = KnxNetIpHeader::new(
            KnxNetIpServiceType::RoutingLostMessage,
            (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (msg_body.len() as u16),
        );
        let mut packet = header.to_buffer();
        packet.extend_from_slice(&msg_body);
        let dest = format!("{}:{}", options.ip, options.port);
        if let Ok(dest_addr) = dest.parse::<SocketAddr>() {
            let socket_cloned = Arc::clone(socket);
            tokio::spawn(async move {
                let _ = socket_cloned.send_to(&packet[..], &dest_addr).await;
            });
        }
    }

    fn send_routing_busy(socket: &Arc<UdpSocket>, options: &KnxNetIpServerOptions, wait_time: u16) {
        let busy_msg = RoutingBusy::new(0, wait_time, 0x0000);
        let msg_body = busy_msg.to_buffer();
        let header = KnxNetIpHeader::new(
            KnxNetIpServiceType::RoutingBusy,
            (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (msg_body.len() as u16),
        );
        let mut packet = header.to_buffer();
        packet.extend_from_slice(&msg_body);
        let dest = format!("{}:{}", options.ip, options.port);
        if let Ok(dest_addr) = dest.parse::<SocketAddr>() {
            let socket_cloned = Arc::clone(socket);
            tokio::spawn(async move {
                let _ = socket_cloned.send_to(&packet[..], &dest_addr).await;
            });
        }
    }

    fn handle_routing_busy(&self, _socket: &Arc<UdpSocket>, busy: RoutingBusy) {
        let mut pacing = self.multicast_pacing.lock().unwrap();
        let now = Instant::now();
        if now.duration_since(pacing.last_busy_time).as_millis() > 10 {
            pacing.busy_counter += 1;

            let pacing_cloned = Arc::clone(&self.multicast_pacing);
            let delay_ms = (pacing.busy_counter * 100) as u64;
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                loop {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    let mut p = pacing_cloned.lock().unwrap();
                    if p.busy_counter > 0 {
                        p.busy_counter -= 1;
                    } else {
                        break;
                    }
                }
            });
        }
        pacing.last_busy_time = now;

        if busy.routing_busy_control == 0x0000 {
            let rand_val = if pacing.busy_counter > 0 {
                let pseudo_rand = (now.elapsed().as_nanos() % 100) as u32;
                (pseudo_rand * pacing.busy_counter * 50 / 100) as u64
            } else {
                0
            };
            let wait_time = (busy.wait_time as u64) + rand_val;

            pacing.is_routing_busy = true;
            let _ = self.event_tx.send(ServerEvent::RoutingBusy(true));
            let pacing_cloned = Arc::clone(&self.multicast_pacing);
            let notify_cloned = Arc::clone(&self.pacing_notify);
            let event_tx_cloned = self.event_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(wait_time)).await;
                let mut p = pacing_cloned.lock().unwrap();
                p.is_routing_busy = false;
                let _ = event_tx_cloned.send(ServerEvent::RoutingBusy(false));
                let _ = event_tx_cloned.send(ServerEvent::RoutingReady);
                notify_cloned.notify_one();
            });
        }
    }

    pub async fn send_raw(&self, cemi_bytes: &[u8]) -> Result<(), KnxError> {
        let cemi = Cemi::from_buffer(cemi_bytes)?;
        let _ = self.event_tx.send(ServerEvent::Send(cemi.clone()));

        let src_ia_str = match &cemi {
            Cemi::LDataReq(ld) | Cemi::LDataCon(ld) | Cemi::LDataInd(ld) => {
                ld.source_address.clone()
            }
            _ => "".to_string(),
        };

        let busmon_body = convert_data_ind_to_busmon_ind(cemi_bytes);

        let mut sends = Vec::new();
        {
            let mut clients = self.clients.write().unwrap();
            for conn in clients.values_mut() {
                if src_ia_str == conn.knx_address_str {
                    continue;
                }

                if conn.knx_layer == crate::core::knxnetip_enum::KnxLayer::BusmonitorLayer {
                    conn.enqueue(&busmon_body, KnxNetIpServiceType::TunnellingRequest);
                } else {
                    conn.enqueue(cemi_bytes, KnxNetIpServiceType::TunnellingRequest);
                }

                if let Some(packet) = conn.process_queue() {
                    let dest = conn.data_endpoint();
                    if let Ok(dest_addr) = dest.parse::<std::net::SocketAddr>() {
                        sends.push((packet, dest_addr));
                    }
                }
            }
        }

        for (packet, dest_addr) in sends {
            let _ = self.socket_tx.send((packet, dest_addr)).await;
        }

        if self.options.is_routing {
            let mut routing_cemi = cemi_bytes.to_vec();
            let msg_code = routing_cemi[0];
            if msg_code == 0x11 {
                routing_cemi[0] = 0x29;
            } else if msg_code == 0x10 {
                routing_cemi[0] = 0x2d;
            }

            let socket_opt = {
                let socket_guard = self.udp_socket.read().unwrap();
                socket_guard.clone()
            };
            if let Some(socket) = socket_opt {
                self.enqueue_packet(&socket, &routing_cemi).await;
            }
        }

        Ok(())
    }
}

impl KnxService for KnxNetIpServer {
    async fn connect(&self) -> Result<(), KnxError> {
        {
            let mut s = self.state.write().unwrap();
            if *s == KnxServerState::Running {
                return Ok(());
            }
            let old_state = *s;
            *s = KnxServerState::Starting;
            self.logger.info(
                &format!("FSM: State transition from {:?} to {:?}", old_state, *s).to_uppercase(),
            );
        }

        let addr = format!("0.0.0.0:{}", self.options.port);
        let socket = UdpSocket::bind(&addr).await.map_err(|e| {
            self.logger.error(&format!(
                "Failed to bind to UDP port {}: {:?}",
                self.options.port, e
            ));
            KnxError::Io(e.to_string())
        })?;

        let socket = Arc::new(socket);
        {
            let mut socket_guard = self.udp_socket.write().unwrap();
            *socket_guard = Some(Arc::clone(&socket));
        }

        let mcast_ip = Ipv4Addr::from_str(&self.options.ip)
            .map_err(|e| KnxError::Protocol(format!("Invalid multicast IP: {}", e)))?;
        
        let mut joined_interfaces = std::collections::HashSet::new();
        let primary_local_ip =
            Ipv4Addr::from_str(&self.options.local_ip).unwrap_or(Ipv4Addr::new(0, 0, 0, 0));

        if !primary_local_ip.is_unspecified() {
            match socket.join_multicast_v4(mcast_ip, primary_local_ip) {
                Ok(_) => {
                    joined_interfaces.insert(primary_local_ip);
                    self.logger.info(&format!("Joined multicast on primary interface ({})", primary_local_ip));
                }
                Err(e) => {
                    self.logger.warn(&format!("Failed to join multicast on primary interface {}: {:?}", primary_local_ip, e));
                }
            }
        }

        // Try to join multicast on all other interfaces
        if let Ok(interfaces) = if_addrs::get_if_addrs() {
            for iface in interfaces {
                if !iface.is_loopback() {
                    if let std::net::IpAddr::V4(ipv4_addr) = iface.ip() {
                        if !joined_interfaces.contains(&ipv4_addr) {
                            match socket.join_multicast_v4(mcast_ip, ipv4_addr) {
                                Ok(_) => {
                                    joined_interfaces.insert(ipv4_addr);
                                    self.logger.info(&format!("Joined multicast on interface {} ({})", iface.name, ipv4_addr));
                                }
                                Err(_) => {
                                    // Ignore virtual/non-multicast interfaces
                                }
                            }
                        }
                    }
                }
            }
        }

        if joined_interfaces.is_empty() {
            match socket.join_multicast_v4(mcast_ip, Ipv4Addr::new(0, 0, 0, 0)) {
                Ok(_) => {
                    self.logger.info("Joined multicast on fallback 0.0.0.0");
                }
                Err(e) => {
                    {
                        let mut s = self.state.write().unwrap();
                        let old = *s;
                        *s = KnxServerState::Faulted;
                        self.logger.info(
                            &format!("FSM: State transition from {:?} to {:?}", old, *s).to_uppercase(),
                        );
                    }
                    return Err(KnxError::Protocol(format!(
                        "Failed to join multicast group on any interface: {:?}", e
                    )));
                }
            }
        }

        let _ = socket.set_multicast_loop_v4(true);

        let recv_socket = Arc::clone(&socket);
        let mut rx = self.socket_rx.lock().await.take().unwrap();

        let pacing_task = Arc::clone(&self.multicast_pacing);
        let socket_pacing = Arc::clone(&socket);
        let options_pacing = self.options.clone();
        let notify_pacing = Arc::clone(&self.pacing_notify);

        // Queue pacing task
        tokio::spawn(async move {
            loop {
                notify_pacing.notified().await;

                loop {
                    let (packet, next_wait) = {
                        let mut pacing = pacing_task.lock().unwrap();
                        if pacing.is_routing_busy || pacing.msg_queue.is_empty() {
                            break;
                        }

                        let packet = pacing.msg_queue.pop_front().unwrap();
                        let delay = options_pacing.routing_delay as u64;
                        let now = Instant::now();
                        let elapsed = now.duration_since(pacing.last_sent_time).as_millis() as u64;
                        let next_wait = if elapsed < delay { delay - elapsed } else { 0 };
                        (packet, next_wait)
                    };

                    if next_wait > 0 {
                        tokio::time::sleep(Duration::from_millis(next_wait)).await;
                    }

                    let dest = format!("{}:{}", options_pacing.ip, options_pacing.port);
                    if let Ok(dest_addr) = dest.parse::<SocketAddr>() {
                        let _ = socket_pacing.send_to(&packet[..], &dest_addr).await;
                    }

                    {
                        let mut pacing = pacing_task.lock().unwrap();
                        pacing.last_sent_time = Instant::now();
                    }
                }
            }
        });

        // Periodic Check/Heartbeat task
        let clients_check = Arc::clone(&self.clients);
        let socket_check = Arc::clone(&socket);
        let options_check = self.options.clone();
        let logger = self.logger.clone();
        let event_tx_check = self.event_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;

                let mut sends = Vec::new();
                let mut channels_to_remove = Vec::new();
                {
                    let mut clients = clients_check.write().unwrap();
                    for (&channel_id, conn) in clients.iter_mut() {
                        if conn.is_heartbeat_expired() {
                            channels_to_remove.push((channel_id, true));
                        } else if conn.is_ack_timeout() {
                            if conn.pending_ack_is_retransmission() == Some(true) {
                                if !options_check.ignore_acktimeout {
                                    logger.error(&format!("Second ACK timeout for channel_id {channel_id}. Terminating connection."));
                                    channels_to_remove.push((channel_id, true));
                                } else {
                                    logger.warn(&format!("Second ACK timeout for channel_id {channel_id}. Continuing anyway."));
                                    conn.ignore_ack_timeout();
                                }
                            } else {
                                if let Some(packet) = conn.retransmit() {
                                    logger.warn(&format!(
                                        "ACK timeout for channel_id {channel_id}, retransmitting..."
                                    ));
                                    sends.push((packet, conn.data_endpoint()));
                                }
                            }
                        }

                        if let Some(packet) = conn.process_queue() {
                            sends.push((packet, conn.data_endpoint()));
                        }
                    }
                }

                for (packet, endpoint) in sends {
                    if let Ok(dest_addr) = endpoint.parse::<SocketAddr>() {
                        let _ = socket_check.send_to(&packet[..], &dest_addr).await;
                    }
                }

                for (channel_id, send_disconnect) in channels_to_remove {
                    let mut conn_to_close = None;
                    {
                        let mut clients = clients_check.write().unwrap();
                        if let Some(mut conn) = clients.remove(&channel_id) {
                            conn.close();
                            conn_to_close = Some(conn);
                            let _ =
                                event_tx_check.send(ServerEvent::DisconnectedClient(channel_id));
                        }
                    }

                    if let Some(conn) = conn_to_close {
                        if send_disconnect {
                            let hpai = Hpai::new(
                                HostProtocolCode::Ipv4Udp,
                                Ipv4Addr::from_str(&options_check.local_ip)
                                    .unwrap_or(Ipv4Addr::new(127, 0, 0, 1)),
                                options_check.port,
                            );
                            let mut body = vec![channel_id, 0x00];
                            body.extend_from_slice(&hpai.to_buffer());
                            let header = KnxNetIpHeader::new(
                                KnxNetIpServiceType::DisconnectRequest,
                                (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (body.len() as u16),
                            );
                            let mut packet = header.to_buffer();
                            packet.extend_from_slice(&body);
                            if let Ok(dest_addr) = conn.control_endpoint().parse::<SocketAddr>() {
                                let _ = socket_check.send_to(&packet[..], &dest_addr).await;
                            }
                        }
                    }
                }
            }
        });

        let ctx = Arc::new(HandlerContext {
            options: self.options.clone(),
            clients: Arc::clone(&self.clients),
            incoming_tx: self.incoming_tx.clone(),
            event_tx: self.event_tx.clone(),
            server_ia_int: self.server_ia_int,
            max_tunnel_connections: self.max_tunnel_connections,
            client_addrs_start_int: self.client_addrs_start_int,
            multicast_pacing: Arc::clone(&self.multicast_pacing),
            pacing_notify: Arc::clone(&self.pacing_notify),
            udp_socket: Arc::clone(&self.udp_socket),
            logger: self.logger.clone(),
        });

        let ctx_cloned = Arc::clone(&ctx);
        let socket_cloned = Arc::clone(&socket);

        // Reader task
        tokio::spawn(async move {
            let mut buf = vec![0u8; 2048];
            loop {
                if let Ok((len, addr)) = recv_socket.recv_from(&mut buf).await {
                    let packet = &buf[..len];
                    let ctx_lh = Arc::clone(&ctx_cloned);
                    let socket_lh = Arc::clone(&socket_cloned);

                    let _ = handle_message_static(packet, addr, &socket_lh, ctx_lh.as_ref()).await;
                }
            }
        });

        // Writer task
        tokio::spawn(async move {
            while let Some((packet, dest)) = rx.recv().await {
                let _ = socket.send_to(&packet[..], &dest).await;
            }
        });

        {
            let mut s = self.state.write().unwrap();
            let old_state = *s;
            *s = KnxServerState::Running;
            self.logger.info(
                &format!("FSM: State transition from {:?} to {:?}", old_state, *s).to_uppercase(),
            );
        }
        let _ = self.event_tx.send(ServerEvent::Connected);
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), KnxError> {
        let mut s = self.state.write().unwrap();
        if *s == KnxServerState::Stopped {
            return Ok(());
        }
        let old_state = *s;
        *s = KnxServerState::Stopped;
        self.logger.info(
            &format!("FSM: State transition from {:?} to {:?}", old_state, *s).to_uppercase(),
        );

        self.logger.info("Stopping KNXnet/IP server...");

        {
            let mut socket_guard = self.udp_socket.write().unwrap();
            *socket_guard = None;
        }

        let mut clients = self.clients.write().unwrap();
        for mut conn in clients.drain() {
            conn.1.close();
        }

        let mut pacing = self.multicast_pacing.lock().unwrap();
        pacing.msg_queue.clear();

        self.logger.info("KNXnet/IP server stopped.");
        let _ = self.event_tx.send(ServerEvent::Disconnected);

        Ok(())
    }

    async fn send(&self, cemi: &Cemi) -> Result<(), KnxError> {
        let cemi_bytes = cemi.to_buffer();
        self.send_raw(&cemi_bytes).await
    }

    fn connection_state(&self) -> String {
        match *self.state.read().unwrap() {
            KnxServerState::Stopped => "STOPPED".to_string(),
            KnxServerState::Starting => "STARTING".to_string(),
            KnxServerState::Running => "RUNNING".to_string(),
            KnxServerState::Faulted => "FAULTED".to_string(),
        }
    }

    fn is_connected(&self) -> bool {
        *self.state.read().unwrap() == KnxServerState::Running
    }

    fn individual_address(&self) -> String {
        self.options.individual_address.clone()
    }
}

// Lightweight static handler helper context
struct HandlerContext {
    options: KnxNetIpServerOptions,
    clients: Arc<RwLock<HashMap<u8, TunnelConnection>>>,
    incoming_tx: broadcast::Sender<Cemi>,
    event_tx: broadcast::Sender<ServerEvent>,
    server_ia_int: u16,
    max_tunnel_connections: u8,
    client_addrs_start_int: u16,
    multicast_pacing: Arc<std::sync::Mutex<MulticastPacing>>,
    pacing_notify: Arc<tokio::sync::Notify>,
    #[allow(dead_code)]
    udp_socket: Arc<RwLock<Option<Arc<UdpSocket>>>>,
    logger: Logger,
}

async fn handle_message_static(
    msg: &[u8],
    rinfo: SocketAddr,
    socket: &Arc<UdpSocket>,
    ctx: &HandlerContext,
) -> Result<(), KnxError> {
    if msg.len() < KnxNetIpHeader::HEADER_SIZE_10.into() {
        return Err(KnxError::InvalidParametersForDpt);
    }

    let header = KnxNetIpHeader::from_buffer(msg)?;
    let body = &msg[KnxNetIpHeader::HEADER_SIZE_10 as usize..];

    // Anti-Echo Check by IP/Port
    let local_ip = Ipv4Addr::from_str(&ctx.options.local_ip).unwrap_or(Ipv4Addr::new(127, 0, 0, 1));
    if let SocketAddr::V4(addr_v4) = rinfo {
        if *addr_v4.ip() == local_ip && addr_v4.port() == ctx.options.port {
            return Ok(());
        }
    }

    match header.service_type {
        KnxNetIpServiceType::SearchRequest | KnxNetIpServiceType::SearchRequestExtended => {
            let is_extended = header.service_type == KnxNetIpServiceType::SearchRequestExtended;
            let mut client_hpai = Hpai::from_buffer(body)?;
            if !KnxNetIpServer::resolve_route_back(&mut client_hpai, rinfo) {
                return Ok(());
            }

            let mut s_ip =
                Ipv4Addr::from_str(&ctx.options.local_ip).unwrap_or(Ipv4Addr::new(127, 0, 0, 1));
            if s_ip.is_unspecified() {
                if let Some(ip) = get_local_ip_routing_to(rinfo.ip()) {
                    s_ip = ip;
                } else if let Some(ip) = get_local_ip() {
                    s_ip = ip;
                }
            }
            let server_hpai = Hpai::new(HostProtocolCode::Ipv4Udp, s_ip, ctx.options.port);

            let response_type = if is_extended {
                KnxNetIpServiceType::SearchResponseExtended
            } else {
                KnxNetIpServiceType::SearchResponse
            };

            let dibs = get_identification_dibs_static(ctx, response_type, server_hpai.ip_address);

            let mut res_body = server_hpai.to_buffer();
            for dib in dibs {
                res_body.extend_from_slice(&dib.to_buffer());
            }

            let res_header = KnxNetIpHeader::new(
                response_type,
                (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
            );

            let mut packet = res_header.to_buffer();
            packet.extend_from_slice(&res_body);

            let dest = SocketAddr::new(
                std::net::IpAddr::V4(client_hpai.ip_address),
                client_hpai.port,
            );
            let _ = socket.send_to(&packet[..], &dest).await;
        }

        KnxNetIpServiceType::DescriptionRequest => {
            let mut client_hpai = Hpai::from_buffer(body)?;
            if !KnxNetIpServer::resolve_route_back(&mut client_hpai, rinfo) {
                return Ok(());
            }

            let mut s_ip =
                Ipv4Addr::from_str(&ctx.options.local_ip).unwrap_or(Ipv4Addr::new(127, 0, 0, 1));
            if s_ip.is_unspecified() {
                if let Some(ip) = get_local_ip_routing_to(rinfo.ip()) {
                    s_ip = ip;
                } else if let Some(ip) = get_local_ip() {
                    s_ip = ip;
                }
            }
            let server_hpai = Hpai::new(HostProtocolCode::Ipv4Udp, s_ip, ctx.options.port);

            let dibs = get_identification_dibs_static(
                ctx,
                KnxNetIpServiceType::DescriptionResponse,
                server_hpai.ip_address,
            );

            let mut res_body = Vec::new();
            for dib in dibs {
                res_body.extend_from_slice(&dib.to_buffer());
            }

            let res_header = KnxNetIpHeader::new(
                KnxNetIpServiceType::DescriptionResponse,
                (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
            );

            let mut packet = res_header.to_buffer();
            packet.extend_from_slice(&res_body);

            let dest = SocketAddr::new(
                std::net::IpAddr::V4(client_hpai.ip_address),
                client_hpai.port,
            );
            let _ = socket.send_to(&packet[..], &dest).await;
        }

        KnxNetIpServiceType::ConnectRequest => {
            let mut client_control_hpai = Hpai::from_buffer(&body[..8])?;
            let mut client_data_hpai = Hpai::from_buffer(&body[8..16])?;

            if !KnxNetIpServer::resolve_route_back(&mut client_control_hpai, rinfo) {
                return Ok(());
            }
            if !KnxNetIpServer::resolve_route_back(&mut client_data_hpai, rinfo) {
                return Ok(());
            }

            let cri = Cri::from_buffer(&body[16..])?;
            let mut status = KnxNetIpErrorCodes::ENoError as u8;
            let mut channel_id = 0;

            let mut s_ip =
                Ipv4Addr::from_str(&ctx.options.local_ip).unwrap_or(Ipv4Addr::new(127, 0, 0, 1));
            if s_ip.is_unspecified() {
                if let Some(ip) = get_local_ip_routing_to(rinfo.ip()) {
                    s_ip = ip;
                } else if let Some(ip) = get_local_ip() {
                    s_ip = ip;
                }
            }
            let server_data_hpai = Hpai::new(HostProtocolCode::Ipv4Udp, s_ip, ctx.options.port);

            let mut send_packet = None;
            let dest = SocketAddr::new(
                std::net::IpAddr::V4(client_control_hpai.ip_address),
                client_control_hpai.port,
            );

            {
                let mut clients = ctx.clients.write().unwrap();

                if let Some(addr) = cri.individual_address {
                    if addr != 0 {
                        let mut stale_channel = None;
                        for (&cid, conn) in clients.iter() {
                            if conn.knx_address == addr
                                && conn.control_hpai.ip_address == client_control_hpai.ip_address
                            {
                                stale_channel = Some(cid);
                                break;
                            }
                        }
                        if let Some(cid) = stale_channel {
                            if let Some(mut conn) = clients.remove(&cid) {
                                conn.close();
                                let _ = ctx.event_tx.send(ServerEvent::DisconnectedClient(cid));
                            }
                        }
                    }
                }

                for i in 1..=ctx.max_tunnel_connections {
                    if !clients.contains_key(&i) {
                        channel_id = i;
                        break;
                    }
                }

                if channel_id == 0 {
                    status = KnxNetIpErrorCodes::ENoMoreConnections as u8;
                } else if cri.connection_type == ConnectionType::DeviceMgmtConnection {
                    let conn = TunnelConnection::new(
                        channel_id,
                        client_control_hpai.clone(),
                        client_data_hpai.clone(),
                        0,
                        "0.0.0".to_string(),
                        crate::core::knxnetip_enum::KnxLayer::from_u8(cri.knx_layer)
                            .unwrap_or(crate::core::knxnetip_enum::KnxLayer::LinkLayer),
                        120_000,
                        1_000,
                        100,
                        ctx.options.ignore_acktimeout,
                    );
                    clients.insert(channel_id, conn);

                    let mut res_body = vec![channel_id, status];
                    res_body.extend_from_slice(&server_data_hpai.to_buffer());
                    res_body.extend_from_slice(&[0x02, ConnectionType::DeviceMgmtConnection as u8]);

                    let res_header = KnxNetIpHeader::new(
                        KnxNetIpServiceType::ConnectResponse,
                        (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
                    );
                    let mut packet = res_header.to_buffer();
                    packet.extend_from_slice(&res_body);
                    send_packet = Some(packet);
                } else if cri.connection_type == ConnectionType::TunnelConnection {
                    let mut knx_addr = cri.individual_address.unwrap_or(0);
                    if knx_addr == 0 {
                        knx_addr = ctx.client_addrs_start_int + (channel_id as u16) - 1;
                    }

                    if status == KnxNetIpErrorCodes::ENoError as u8 {
                        for (_, conn) in clients.iter() {
                            if conn.knx_address == knx_addr {
                                status = KnxNetIpErrorCodes::ENoMoreUniqueConnections as u8;
                                break;
                            }
                        }
                    }

                    if status == KnxNetIpErrorCodes::ENoError as u8 {
                        let knx_address_str =
                            KnxHelper::get_address_from_number(knx_addr, ".", false)
                                .unwrap_or_else(|_| "0.0.0".to_string());

                        let conn = TunnelConnection::new(
                            channel_id,
                            client_control_hpai.clone(),
                            client_data_hpai.clone(),
                            knx_addr,
                            knx_address_str,
                            crate::core::knxnetip_enum::KnxLayer::from_u8(cri.knx_layer)
                                .unwrap_or(crate::core::knxnetip_enum::KnxLayer::LinkLayer),
                            120_000,
                            1_000,
                            100,
                            ctx.options.ignore_acktimeout,
                        );
                        clients.insert(channel_id, conn);

                        let crd = Crd::new(ConnectionType::TunnelConnection, knx_addr);
                        let mut res_body = vec![channel_id, status];
                        res_body.extend_from_slice(&server_data_hpai.to_buffer());
                        res_body.extend_from_slice(&crd.to_buffer());

                        let res_header = KnxNetIpHeader::new(
                            KnxNetIpServiceType::ConnectResponse,
                            (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
                        );
                        let mut packet = res_header.to_buffer();
                        packet.extend_from_slice(&res_body);
                        send_packet = Some(packet);
                    }
                } else {
                    status = KnxNetIpErrorCodes::EConnectionType as u8;
                }

                if send_packet.is_none() {
                    let res_header = KnxNetIpHeader::new(KnxNetIpServiceType::ConnectResponse, 8);
                    let mut packet = res_header.to_buffer();
                    packet.extend_from_slice(&[0, status]);
                    send_packet = Some(packet);
                }
            }

            if let Some(packet) = send_packet {
                let _ = socket.send_to(&packet[..], &dest).await;
            }
        }

        KnxNetIpServiceType::ConnectionStateRequest => {
            let channel_id = body[0];
            let mut client_control_hpai = Hpai::from_buffer(&body[2..])?;
            if !KnxNetIpServer::resolve_route_back(&mut client_control_hpai, rinfo) {
                return Ok(());
            }

            let mut status = KnxNetIpErrorCodes::EConnectionId as u8;
            {
                let mut clients = ctx.clients.write().unwrap();
                if let Some(conn) = clients.get_mut(&channel_id) {
                    status = KnxNetIpErrorCodes::ENoError as u8;
                    conn.reset_heartbeat();
                }
            }

            let res_body = vec![channel_id, status];
            let res_header = KnxNetIpHeader::new(
                KnxNetIpServiceType::ConnectionStateResponse,
                (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
            );
            let mut packet = res_header.to_buffer();
            packet.extend_from_slice(&res_body);

            let dest = SocketAddr::new(
                std::net::IpAddr::V4(client_control_hpai.ip_address),
                client_control_hpai.port,
            );
            let _ = socket.send_to(&packet[..], &dest).await;
        }

        KnxNetIpServiceType::DisconnectRequest => {
            let channel_id = body[0];
            let mut client_control_hpai = Hpai::from_buffer(&body[2..])?;
            if !KnxNetIpServer::resolve_route_back(&mut client_control_hpai, rinfo) {
                return Ok(());
            }

            {
                let mut clients = ctx.clients.write().unwrap();
                if let Some(mut conn) = clients.remove(&channel_id) {
                    conn.close();
                    let _ = ctx
                        .event_tx
                        .send(ServerEvent::DisconnectedClient(channel_id));
                }
            }

            let res_body = vec![channel_id, KnxNetIpErrorCodes::ENoError as u8];
            let res_header = KnxNetIpHeader::new(
                KnxNetIpServiceType::DisconnectResponse,
                (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (res_body.len() as u16),
            );
            let mut packet = res_header.to_buffer();
            packet.extend_from_slice(&res_body);

            let dest = SocketAddr::new(
                std::net::IpAddr::V4(client_control_hpai.ip_address),
                client_control_hpai.port,
            );
            let _ = socket.send_to(&packet[..], &dest).await;
        }

        KnxNetIpServiceType::TunnellingRequest => {
            let header_len = body[0] as usize;
            let channel_id = body[1];
            let seq = body[2];
            let cemi_bytes = &body[header_len..];

            let mut flood_disconnect = None;
            let (action, status, knx_layer, knx_address, data_endpoint) = {
                let mut clients = ctx.clients.write().unwrap();
                if let Some(conn) = clients.get_mut(&channel_id) {
                    let now = Instant::now();
                    if now.duration_since(conn.last_rx_time).as_secs() >= 1 {
                        conn.rx_count = 0;
                        conn.last_rx_time = now;
                    }
                    conn.rx_count += 1;

                    let flood_threshold = 100;
                    if flood_threshold > 0 && conn.rx_count > flood_threshold {
                        let ep = conn.control_endpoint();
                        conn.close();
                        clients.remove(&channel_id);
                        let _ = ctx
                            .event_tx
                            .send(ServerEvent::DisconnectedClient(channel_id));
                        flood_disconnect = Some(ep);
                        (
                            RequestAction::Discard,
                            0,
                            crate::core::knxnetip_enum::KnxLayer::LinkLayer,
                            0,
                            "".to_string(),
                        )
                    } else {
                        let val = conn.validate_request(seq);
                        (
                            val.action,
                            val.status,
                            conn.knx_layer,
                            conn.knx_address,
                            conn.data_endpoint(),
                        )
                    }
                } else {
                    (
                        RequestAction::RetransmitAck,
                        KnxNetIpErrorCodes::EConnectionId as u8,
                        crate::core::knxnetip_enum::KnxLayer::LinkLayer,
                        0,
                        "".to_string(),
                    )
                }
            };

            if let Some(control_ep) = flood_disconnect {
                let mut s_ip = Ipv4Addr::from_str(&ctx.options.local_ip)
                    .unwrap_or(Ipv4Addr::new(127, 0, 0, 1));
                if s_ip.is_unspecified() {
                    if let Some(ip) = get_local_ip_routing_to(rinfo.ip()) {
                        s_ip = ip;
                    } else if let Some(ip) = get_local_ip() {
                        s_ip = ip;
                    }
                }
                let server_hpai = Hpai::new(HostProtocolCode::Ipv4Udp, s_ip, ctx.options.port);

                let mut dis_body = vec![channel_id, 0x00];
                dis_body.extend_from_slice(&server_hpai.to_buffer());
                let dis_header = KnxNetIpHeader::new(
                    KnxNetIpServiceType::DisconnectRequest,
                    (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (dis_body.len() as u16),
                );
                let mut packet = dis_header.to_buffer();
                packet.extend_from_slice(&dis_body);
                if let Ok(dest) = control_ep.parse::<SocketAddr>() {
                    let _ = socket.send_to(&packet[..], &dest).await;
                }
                return Ok(());
            }

            if action == RequestAction::RetransmitAck {
                if let Ok(dest_addr) = data_endpoint.parse::<SocketAddr>() {
                    send_tunnel_ack_static(channel_id, seq, status, dest_addr, socket, ctx).await;
                }
                return Ok(());
            }
            if action == RequestAction::Discard {
                return Ok(());
            }

            if let Ok(dest_addr) = data_endpoint.parse::<SocketAddr>() {
                send_tunnel_ack_static(channel_id, seq, status, dest_addr, socket, ctx).await;
            }

            let msg_code = cemi_bytes[0];
            let add_info_len = cemi_bytes[1];

            if knx_layer == crate::core::knxnetip_enum::KnxLayer::BusmonitorLayer
                && (msg_code == 0x11 || msg_code == 0x10)
            {
                return Ok(());
            }

            let mut mut_cemi_bytes = cemi_bytes.to_vec();
            if msg_code == 0x11 {
                let src_ia_offset = 2 + add_info_len as usize + 2;
                if src_ia_offset + 1 < mut_cemi_bytes.len() {
                    let src_ia = ((mut_cemi_bytes[src_ia_offset] as u16) << 8)
                        | (mut_cemi_bytes[src_ia_offset + 1] as u16);
                    if src_ia == 0 {
                        mut_cemi_bytes[src_ia_offset] = (knx_address >> 8) as u8;
                        mut_cemi_bytes[src_ia_offset + 1] = (knx_address & 0xFF) as u8;
                    }
                }
            }

            if let Ok(cemi) = Cemi::from_buffer(&mut_cemi_bytes) {
                let _ = ctx.incoming_tx.send(cemi.clone());
                let _ = ctx.event_tx.send(ServerEvent::Indication(cemi.clone()));
                let _ = ctx
                    .event_tx
                    .send(ServerEvent::RawIndication(mut_cemi_bytes.clone()));
                ctx.logger.log_indication(&cemi);
                ctx.logger.log_indication_raw(&mut_cemi_bytes);
                let _ = crate::core::cache::group_address_cache::GroupAddressCache::get_instance()
                    .write()
                    .unwrap()
                    .process_cemi(&cemi);
            }

            let mut routing_cemi = mut_cemi_bytes.clone();
            if msg_code == 0x11 {
                routing_cemi[0] = 0x29; // L_Data.ind
            } else if msg_code == 0x10 {
                routing_cemi[0] = 0x2d; // L_Poll_Data.con
            }

            enqueue_packet_static(socket, ctx, &routing_cemi).await;

            if msg_code == 0x11 || msg_code == 0x10 {
                let mut con_cemi = mut_cemi_bytes.clone();
                con_cemi[0] = msg_code + 0x1D;
                con_cemi[2 + add_info_len as usize] &= 0xFE;

                let mut send_packet = None;
                let mut endpoint = None;
                {
                    let mut clients = ctx.clients.write().unwrap();
                    if let Some(conn) = clients.get_mut(&channel_id) {
                        conn.enqueue(&con_cemi, KnxNetIpServiceType::TunnellingRequest);
                        send_packet = conn.process_queue();
                        endpoint = Some(conn.data_endpoint());
                    }
                }
                if let (Some(pkt), Some(ep)) = (send_packet, endpoint) {
                    if let Ok(dest) = ep.parse::<SocketAddr>() {
                        let _ = socket.send_to(&pkt[..], &dest).await;
                    }
                }
            }
        }

        KnxNetIpServiceType::TunnellingAck => {
            let channel_id = body[1];
            let seq = body[2];
            let status = body[3];

            let mut clients = ctx.clients.write().unwrap();
            if let Some(conn) = clients.get_mut(&channel_id) {
                let _ = conn.handle_ack(seq, status);

                if let Some(packet) = conn.process_queue() {
                    let dest = conn.data_endpoint();
                    let socket_cloned = Arc::clone(socket);
                    tokio::spawn(async move {
                        let _ = socket_cloned.send_to(&packet[..], &dest).await;
                    });
                }
            }
        }

        KnxNetIpServiceType::DeviceConfigurationRequest => {
            let header_len = body[0] as usize;
            let channel_id = body[1];
            let seq = body[2];
            let cemi_bytes = &body[header_len..];

            let (action, status, data_endpoint) = {
                let mut clients = ctx.clients.write().unwrap();
                if let Some(conn) = clients.get_mut(&channel_id) {
                    let val = conn.validate_request(seq);
                    (val.action, val.status, conn.data_endpoint())
                } else {
                    (
                        RequestAction::RetransmitAck,
                        KnxNetIpErrorCodes::EConnectionId as u8,
                        "".to_string(),
                    )
                }
            };

            if action == RequestAction::RetransmitAck {
                if let Ok(dest_addr) = data_endpoint.parse::<SocketAddr>() {
                    send_device_config_ack_static(channel_id, seq, status, socket, dest_addr).await;
                }
                return Ok(());
            }
            if action == RequestAction::Discard {
                return Ok(());
            }

            if let Ok(dest_addr) = data_endpoint.parse::<SocketAddr>() {
                send_device_config_ack_static(
                    channel_id,
                    seq,
                    KnxNetIpErrorCodes::ENoError as u8,
                    socket,
                    dest_addr,
                )
                .await;
            }

            if let Ok(cemi) = Cemi::from_buffer(cemi_bytes) {
                if let Cemi::MPropReadReq(req) = cemi {
                    let mut data = Vec::new();
                    if req.interface_object_type == 0 && req.property_id == 1 {
                        if let Ok(addr_buf) =
                            KnxHelper::get_address_from_string(&ctx.options.individual_address)
                        {
                            data = addr_buf.to_vec();
                        }
                    }

                    let res_cemi = Cemi::MPropReadCon(MPropWithPayload {
                        interface_object_type: req.interface_object_type,
                        object_instance: req.object_instance,
                        property_id: req.property_id,
                        number_of_elements: req.number_of_elements,
                        start_index: req.start_index,
                        data,
                    });

                    let mut send_packet = None;
                    let mut endpoint = None;
                    {
                        let mut clients = ctx.clients.write().unwrap();
                        if let Some(conn) = clients.get_mut(&channel_id) {
                            conn.enqueue(
                                &res_cemi.to_buffer(),
                                KnxNetIpServiceType::DeviceConfigurationRequest,
                            );
                            send_packet = conn.process_queue();
                            endpoint = Some(conn.data_endpoint());
                        }
                    }
                    if let (Some(pkt), Some(ep)) = (send_packet, endpoint) {
                        if let Ok(dest) = ep.parse::<SocketAddr>() {
                            let _ = socket.send_to(&pkt[..], &dest).await;
                        }
                    }
                }
            }
        }

        KnxNetIpServiceType::DeviceConfigurationAck => {
            let channel_id = body[1];
            let seq = body[2];
            let status = body[3];

            let mut clients = ctx.clients.write().unwrap();
            if let Some(conn) = clients.get_mut(&channel_id) {
                let _ = conn.handle_ack(seq, status);

                if let Some(packet) = conn.process_queue() {
                    let dest = conn.data_endpoint();
                    let socket_cloned = Arc::clone(socket);
                    tokio::spawn(async move {
                        let _ = socket_cloned.send_to(&packet[..], &dest).await;
                    });
                }
            }
        }

        KnxNetIpServiceType::RoutingIndication => {
            let add_info_len = body[1] as usize;
            if body.len() >= 6 + add_info_len {
                let src_ia =
                    ((body[4 + add_info_len] as u16) << 8) | (body[5 + add_info_len] as u16);
                if src_ia == ctx.server_ia_int {
                    return Ok(());
                }
            }

            let cemi_start = 6;
            if let Ok(cemi) = Cemi::from_buffer(&msg[cemi_start..]) {
                let _ = ctx.incoming_tx.send(cemi.clone());
                let _ = ctx.event_tx.send(ServerEvent::Indication(cemi.clone()));
                let _ = ctx
                    .event_tx
                    .send(ServerEvent::RawIndication(msg[cemi_start..].to_vec()));
                ctx.logger.log_indication(&cemi);
                ctx.logger.log_indication_raw(&msg[cemi_start..]);
                let _ = crate::core::cache::group_address_cache::GroupAddressCache::get_instance()
                    .write()
                    .unwrap()
                    .process_cemi(&cemi);

                let src_ia_str = "".to_string();
                let busmon_body = convert_data_ind_to_busmon_ind(&msg[cemi_start..]);

                let mut clients = ctx.clients.write().unwrap();
                for conn in clients.values_mut() {
                    if src_ia_str == conn.knx_address_str {
                        continue;
                    }
                    if conn.knx_layer == crate::core::knxnetip_enum::KnxLayer::BusmonitorLayer {
                        conn.enqueue(&busmon_body, KnxNetIpServiceType::TunnellingRequest);
                    } else {
                        conn.enqueue(&msg[cemi_start..], KnxNetIpServiceType::TunnellingRequest);
                    }

                    if let Some(packet) = conn.process_queue() {
                        let dest = conn.data_endpoint();
                        let socket_cloned = Arc::clone(socket);
                        tokio::spawn(async move {
                            let _ = socket_cloned.send_to(&packet[..], &dest).await;
                        });
                    }
                }
            }
        }

        KnxNetIpServiceType::RoutingBusy => {
            if let Ok(busy) = RoutingBusy::from_buffer(body) {
                handle_routing_busy_static(socket, ctx, busy);
            }
        }

        KnxNetIpServiceType::RoutingLostMessage => {
            if let Ok(lost) = RoutingLostMessage::from_buffer(body) {
                let cemi = Cemi::LBusmonInd(LBusmon {
                    additional_info: Vec::new(),
                    data: lost.to_buffer(),
                });
                let _ = ctx.incoming_tx.send(cemi.clone());
                let _ = ctx
                    .event_tx
                    .send(ServerEvent::RoutingLostMessage(lost.clone()));
                let _ = ctx.event_tx.send(ServerEvent::Indication(cemi.clone()));
                let _ = ctx.event_tx.send(ServerEvent::RawIndication(body.to_vec()));
                ctx.logger.log_indication(&cemi);
                ctx.logger.log_indication_raw(body);
            }
        }

        _ => {}
    }

    Ok(())
}

async fn send_tunnel_ack_static(
    channel_id: u8,
    seq: u8,
    status: u8,
    dest_addr: SocketAddr,
    socket: &Arc<UdpSocket>,
    _ctx: &HandlerContext,
) {
    let body = vec![0x04, channel_id, seq, status];
    let header = KnxNetIpHeader::new(
        KnxNetIpServiceType::TunnellingAck,
        (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (body.len() as u16),
    );
    let mut packet = header.to_buffer();
    packet.extend_from_slice(&body);

    let _ = socket.send_to(&packet[..], &dest_addr).await;
}

async fn send_device_config_ack_static(
    channel_id: u8,
    seq: u8,
    status: u8,
    socket: &Arc<UdpSocket>,
    dest_addr: SocketAddr,
) {
    let body = vec![0x04, channel_id, seq, status];
    let header = KnxNetIpHeader::new(
        KnxNetIpServiceType::DeviceConfigurationAck,
        (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (body.len() as u16),
    );
    let mut packet = header.to_buffer();
    packet.extend_from_slice(&body);

    let _ = socket.send_to(&packet[..], &dest_addr).await;
}

async fn enqueue_packet_static(socket: &Arc<UdpSocket>, ctx: &HandlerContext, cemi_bytes: &[u8]) {
    let header = KnxNetIpHeader::new(
        KnxNetIpServiceType::RoutingIndication,
        (KnxNetIpHeader::HEADER_SIZE_10 as u16) + (cemi_bytes.len() as u16),
    );
    let mut packet = header.to_buffer();
    packet.extend_from_slice(cemi_bytes);

    let is_busy = {
        let mut pacing = ctx.multicast_pacing.lock().unwrap();
        if pacing.msg_queue.len() >= 100 {
            KnxNetIpServer::send_lost_message(socket, &ctx.options, 1);
            let _ = ctx.event_tx.send(ServerEvent::QueueOverflow);
            return;
        }

        pacing.msg_queue.push_back(packet);

        if pacing.msg_queue.len() >= 15 && !pacing.is_routing_busy {
            let delay = ctx.options.routing_delay as usize;
            let wait_time = (delay * pacing.msg_queue.len()).min(100) as u16;
            KnxNetIpServer::send_routing_busy(socket, &ctx.options, wait_time);
            pacing.is_routing_busy = true;
            let _ = ctx.event_tx.send(ServerEvent::RoutingBusy(true));

            let pacing_cloned = Arc::clone(&ctx.multicast_pacing);
            let notify_cloned = Arc::clone(&ctx.pacing_notify);
            let event_tx_cloned = ctx.event_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(wait_time as u64)).await;
                let mut p = pacing_cloned.lock().unwrap();
                p.is_routing_busy = false;
                let _ = event_tx_cloned.send(ServerEvent::RoutingBusy(false));
                let _ = event_tx_cloned.send(ServerEvent::RoutingReady);
                notify_cloned.notify_one();
            });
        }
        pacing.is_routing_busy
    };

    if !is_busy {
        ctx.pacing_notify.notify_one();
    }
}

fn handle_routing_busy_static(_socket: &Arc<UdpSocket>, ctx: &HandlerContext, busy: RoutingBusy) {
    let mut pacing = ctx.multicast_pacing.lock().unwrap();
    let now = Instant::now();
    if now.duration_since(pacing.last_busy_time).as_millis() > 10 {
        pacing.busy_counter += 1;

        let pacing_cloned = Arc::clone(&ctx.multicast_pacing);
        let delay_ms = (pacing.busy_counter * 100) as u64;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            loop {
                tokio::time::sleep(Duration::from_millis(5)).await;
                let mut p = pacing_cloned.lock().unwrap();
                if p.busy_counter > 0 {
                    p.busy_counter -= 1;
                } else {
                    break;
                }
            }
        });
    }
    pacing.last_busy_time = now;

    if busy.routing_busy_control == 0x0000 {
        let rand_val = if pacing.busy_counter > 0 {
            let pseudo_rand = (now.elapsed().as_nanos() % 100) as u32;
            (pseudo_rand * pacing.busy_counter * 50 / 100) as u64
        } else {
            0
        };
        let wait_time = (busy.wait_time as u64) + rand_val;

        pacing.is_routing_busy = true;
        let _ = ctx.event_tx.send(ServerEvent::RoutingBusy(true));
        let pacing_cloned = Arc::clone(&ctx.multicast_pacing);
        let notify_cloned = Arc::clone(&ctx.pacing_notify);
        let event_tx_cloned = ctx.event_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(wait_time)).await;
            let mut p = pacing_cloned.lock().unwrap();
            p.is_routing_busy = false;
            let _ = event_tx_cloned.send(ServerEvent::RoutingBusy(false));
            let _ = event_tx_cloned.send(ServerEvent::RoutingReady);
            notify_cloned.notify_one();
        });
    }
}

fn get_identification_dibs_static(
    ctx: &HandlerContext,
    service_type: KnxNetIpServiceType,
    effective_local_ip: Ipv4Addr,
) -> Vec<Dib> {
    let individual_address = KnxHelper::get_address_from_string(&ctx.options.individual_address)
        .map(|buf| ((buf[0] as u16) << 8) | buf[1] as u16)
        .unwrap_or(0);

    let serial_number = {
        let mut sn = [0u8; 6];
        if let Some(ref s) = ctx.options.serial_number {
            let limit = s.len().min(6);
            sn[..limit].copy_from_slice(&s[..limit]);
        }
        sn
    };

    let mac_address = parse_mac(&ctx.options.mac_address).unwrap_or([0u8; 6]);
    let routing_multicast_address =
        Ipv4Addr::from_str(&ctx.options.ip).unwrap_or(Ipv4Addr::new(224, 0, 23, 12));

    let dev_info = DeviceInformationDib {
        knx_medium: crate::core::knxnetip_enum::KnxMedium::KnxIp,
        device_status: 0,
        individual_address,
        project_installation_id: 0,
        serial_number,
        routing_multicast_address,
        mac_address,
        friendly_name: ctx.options.friendly_name.clone(),
    };

    let supp_svc = SupportedServicesDib {
        services: vec![
            crate::core::knxnetip_structures::SupportedService {
                family: crate::core::knxnetip_enum::AllowedSupportedServiceFamilies::Core as u8,
                version: 1,
            },
            crate::core::knxnetip_structures::SupportedService {
                family:
                    crate::core::knxnetip_enum::AllowedSupportedServiceFamilies::DeviceManagement
                        as u8,
                version: 1,
            },
            crate::core::knxnetip_structures::SupportedService {
                family: crate::core::knxnetip_enum::AllowedSupportedServiceFamilies::Tunnelling
                    as u8,
                version: 1,
            },
            crate::core::knxnetip_structures::SupportedService {
                family: crate::core::knxnetip_enum::AllowedSupportedServiceFamilies::Routing as u8,
                version: 1,
            },
        ],
    };

    if service_type == KnxNetIpServiceType::SearchResponse {
        return vec![Dib::DeviceInfo(dev_info), Dib::SupportedServices(supp_svc)];
    }

    let device_descriptor_type0 = DeviceDescriptorType0::new(
        crate::core::device_descriptor_type::DeviceDescriptorType0::KNXNET_IP_ROUTER.value(),
    );
    let ext_dev_info = ExtendedDeviceInformationDib {
        medium_status: false,
        maximal_local_apdu_length: 254,
        device_descriptor_type0,
    };

    let subnet_mask = Ipv4Addr::new(255, 255, 255, 0);

    let ip_config = IpConfigDib {
        ip_address: effective_local_ip,
        subnet_mask,
        default_gateway: Ipv4Addr::new(0, 0, 0, 0),
        ip_capabilities: 0x01,
        ip_assignment_method: 0x02,
    };

    let ip_current = IpCurrentConfigDib {
        ip_address: effective_local_ip,
        subnet_mask,
        default_gateway: Ipv4Addr::new(0, 0, 0, 0),
        dhcp_server: Ipv4Addr::new(0, 0, 0, 0),
        ip_assignment_method: 0x02,
    };

    let mut slots = Vec::new();
    let clients = ctx.clients.read().unwrap();
    for i in 1..=ctx.max_tunnel_connections {
        let conn = clients.get(&i);
        let mut status = StatusTunnelingSlot::default();
        status.set_authorised(true);
        status.set_usable(conn.is_some());
        status.set_free(conn.is_none());
        slots.push(TunnelSlot {
            address: conn
                .map(|c| c.knx_address)
                .unwrap_or(ctx.client_addrs_start_int + (i as u16) - 1),
            status,
        });
    }

    let tunnelling_info = TunnellingInfoDib {
        apdu_length: 254,
        slots,
    };

    vec![
        Dib::DeviceInfo(dev_info),
        Dib::SupportedServices(supp_svc),
        Dib::ExtendedDeviceInfo(ext_dev_info),
        Dib::IpConfig(ip_config),
        Dib::IpCurrentConfig(ip_current),
        Dib::TunnellingInfo(tunnelling_info),
    ]
}

// Global helper to query local interface IPv4 address
fn get_local_ip() -> Option<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(v4) => Some(v4),
        _ => None,
    }
}

fn get_local_ip_routing_to(dest: std::net::IpAddr) -> Option<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect(SocketAddr::new(dest, 1)).ok()?;
    match socket.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(v4) => Some(v4),
        _ => None,
    }
}

// Global helper to parse MAC address "00:11:22:33:44:55"
fn parse_mac(mac_str: &str) -> Result<[u8; 6], KnxError> {
    let mut mac = [0u8; 6];
    let cleaned = mac_str.replace('-', ":");
    let parts: Vec<&str> = cleaned.split(':').collect();
    if parts.len() != 6 {
        return Err(KnxError::InvalidParametersForDpt);
    }
    for i in 0..6 {
        mac[i] = u8::from_str_radix(parts[i], 16).map_err(|_| KnxError::InvalidParametersForDpt)?;
    }
    Ok(mac)
}

// Converts a cemi frame to LBusmon payload
fn convert_data_ind_to_busmon_ind(cemi_buffer: &[u8]) -> Vec<u8> {
    let msg_code = cemi_buffer[0];
    if msg_code != 0x29
        && msg_code != 0x2d
        && msg_code != 0x2e
        && msg_code != 0x11
        && msg_code != 0x10
    {
        return cemi_buffer.to_vec();
    }
    let add_info_len = cemi_buffer[1] as usize;
    let base_offset = 2 + add_info_len;
    if base_offset + 7 > cemi_buffer.len() {
        return cemi_buffer.to_vec();
    }
    let cf1 = cemi_buffer[base_offset];
    let cf2 = cemi_buffer[base_offset + 1];
    let src = &cemi_buffer[base_offset + 2..base_offset + 4];
    let dst = &cemi_buffer[base_offset + 4..base_offset + 6];
    let data_len = cemi_buffer[base_offset + 6] as usize;
    let tpdu = &cemi_buffer[base_offset + 7..];

    let mut lpdu = Vec::new();
    lpdu.push(cf1);
    lpdu.extend_from_slice(src);
    lpdu.extend_from_slice(dst);
    lpdu.push((cf2 & 0xF0) | ((data_len + 1) as u8));
    lpdu.extend_from_slice(tpdu);
    lpdu.push(0);

    let mut xor = 0u8;
    for i in 0..lpdu.len() - 1 {
        xor ^= lpdu[i];
    }
    let last = lpdu.len() - 1;
    lpdu[last] = !xor;

    Cemi::LBusmonInd(LBusmon {
        additional_info: Vec::new(),
        data: lpdu,
    })
    .to_buffer()
}
