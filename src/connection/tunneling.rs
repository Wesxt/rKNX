use std::sync::Arc;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;
use tokio::net::{UdpSocket, TcpStream};
use tokio::sync::{broadcast, mpsc, oneshot};
use std::sync::RwLock;
use tokio::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::core::cemi::Cemi;
use crate::core::knxnetip_header::KnxNetIpHeader;
use crate::core::knxnetip_enum::{KnxNetIpServiceType, HostProtocolCode, ConnectionType, KnxNetIpErrorCodes};
use crate::core::knxnetip_structures::{Hpai, Cri, Crd};
use crate::errors::KnxError;
use crate::utils::knx_helper::KnxHelper;
use crate::core::cache::group_address_cache::GroupAddressCache;
use super::KnxService;
use crate::utils::logger::Logger;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportProtocol {
    Udp,
    Tcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Faulted,
}

#[derive(Debug, Clone)]
pub struct TunnelingOptions {
    pub ip: String,
    pub port: u16,
    pub local_ip: Option<String>,
    pub local_port: u16,
    pub transport: TransportProtocol,
    pub connection_type: ConnectionType,
    pub use_route_back: bool,
    pub max_queue_size: usize,
    pub auto_reconnect: bool,
    pub max_reconnect_attempts: usize,
    pub reconnect_delay_ms: u64,
}

pub enum ActorMessage {
    SendCemi(Cemi, oneshot::Sender<Result<(), KnxError>>),
    Disconnect,
}

/// Handles KNXnet/IP Tunneling connections for point-to-point communication with a KNX gateway.
/// This class manages the connection state via an internal actor, sequence numbering for reliable delivery,
/// heartbeat monitoring (ConnectionState), and message queuing over both UDP and TCP transports.
pub struct KnxTunneling {
    pub options: TunnelingOptions,
    state: Arc<RwLock<TunnelState>>,
    individual_address: Arc<RwLock<String>>,
    actor_tx: Arc<tokio::sync::Mutex<Option<mpsc::Sender<ActorMessage>>>>,
    incoming_tx: broadcast::Sender<Cemi>,
    logger: Logger,
}

impl KnxTunneling {
    pub fn new(options: TunnelingOptions) -> Self {
        let (incoming_tx, _) = broadcast::channel(100);
        let logger = Logger::new("KNXTunneling");
        Self {
            options,
            state: Arc::new(RwLock::new(TunnelState::Disconnected)),
            individual_address: Arc::new(RwLock::new("1.0.1".to_string())),
            actor_tx: Arc::new(tokio::sync::Mutex::new(None)),
            incoming_tx,
            logger,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Cemi> {
        self.incoming_tx.subscribe()
    }
}

// Internal connection handle to abstract UDP vs TCP operations
enum SocketType {
    Udp(Arc<UdpSocket>),
    Tcp(tokio::io::ReadHalf<TcpStream>, tokio::io::WriteHalf<TcpStream>),
}

impl KnxService for KnxTunneling {
    async fn connect(&self) -> Result<(), KnxError> {
        if *self.state.read().unwrap() == TunnelState::Connected {
            return Ok(());
        }

        self.logger.info(&format!(
            "Tunneling initialized with ip: {} and port: {} and transport: {:?}",
            self.options.ip, self.options.port, self.options.transport
        ));

        let (conn_done_tx, conn_done_rx) = oneshot::channel();
        let (actor_tx, mut actor_rx) = mpsc::channel(self.options.max_queue_size);
        
        // Save actor sender
        {
            let mut guard = self.actor_tx.lock().await;
            *guard = Some(actor_tx);
        }

        let options = self.options.clone();
        let state = self.state.clone();
        let individual_address = self.individual_address.clone();
        let _incoming_tx = self.incoming_tx.clone();
        let logger = self.logger.clone();

        {
            let mut s = state.write().unwrap();
            *s = TunnelState::Connecting;
        }

        // Spawn background task
        tokio::spawn(async move {
            let mut conn_done_tx = Some(conn_done_tx);
            let mut attempts = 0;

            loop {
                let host_addr = format!("{}:{}", options.ip, options.port);
                
                // Connect socket
                let socket_res = if options.transport == TransportProtocol::Tcp {
                    TcpStream::connect(&host_addr).await
                        .map_err(|e| KnxError::Io(e.to_string()))
                        .map(|s| {
                            let (rh, wh) = tokio::io::split(s);
                            SocketType::Tcp(rh, wh)
                        })
                } else {
                    let local = format!(
                        "{}:{}",
                        options.local_ip.as_deref().unwrap_or("0.0.0.0"),
                        options.local_port
                    );
                    UdpSocket::bind(&local).await
                        .map_err(|e| KnxError::Io(e.to_string()))
                        .map(|s| SocketType::Udp(Arc::new(s)))
                };

                let mut socket = match socket_res {
                    Ok(s) => s,
                    Err(e) => {
                        logger.error(&format!("Connection error: {:?}", e));
                        if let Some(tx) = conn_done_tx.take() {
                            let _ = tx.send(Err(e));
                        }
                        if options.auto_reconnect && attempts < options.max_reconnect_attempts {
                            attempts += 1;
                            logger.info(&format!("Reconnecting attempt {}/{}...", attempts, options.max_reconnect_attempts));
                            {
                                let mut s = state.write().unwrap();
                                *s = TunnelState::Reconnecting;
                            }
                            tokio::time::sleep(Duration::from_millis(options.reconnect_delay_ms)).await;
                            continue;
                        } else {
                            logger.error("Connection failed. Maximum attempts reached or auto-reconnect disabled.");
                            {
                                let mut s = state.write().unwrap();
                                *s = TunnelState::Faulted;
                            }
                            break;
                        }
                    }
                };

                // Prepare connect request
                let local_ip_parsed = Ipv4Addr::from_str(options.local_ip.as_deref().unwrap_or("0.0.0.0")).unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
                let local_port_val = match &socket {
                    SocketType::Udp(s) => s.local_addr().map(|a| a.port()).unwrap_or(0),
                    SocketType::Tcp(_, _) => 0, // TCP port assigned dynamically
                };

                let hpai = Hpai::new(
                    if options.transport == TransportProtocol::Tcp { HostProtocolCode::Ipv4Tcp } else { HostProtocolCode::Ipv4Udp },
                    if options.use_route_back { Ipv4Addr::new(0, 0, 0, 0) } else { local_ip_parsed },
                    if options.use_route_back { 0 } else { local_port_val },
                );
                let cri = Cri::new(options.connection_type, crate::core::knxnetip_enum::KnxLayer::LinkLayer as u8, None);

                let mut body = hpai.to_buffer();
                body.extend_from_slice(&hpai.to_buffer());
                body.extend_from_slice(&cri.to_buffer());

                let header = KnxNetIpHeader::new(KnxNetIpServiceType::ConnectRequest, (6 + body.len()) as u16);
                let mut packet = header.to_buffer();
                packet.extend_from_slice(&body);

                // Send request
                let send_res = match &mut socket {
                    SocketType::Udp(s) => s.send_to(&packet, &host_addr).await.map(|_| ()).map_err(|_| ()),
                    SocketType::Tcp(_, wh) => wh.write_all(&packet).await.map_err(|_| ()),
                };

                if send_res.is_err() {
                    logger.error("Failed to send ConnectRequest packet.");
                    if options.auto_reconnect && attempts < options.max_reconnect_attempts {
                        attempts += 1;
                        logger.info(&format!("Reconnecting attempt {}/{}...", attempts, options.max_reconnect_attempts));
                        {
                            let mut s = state.write().unwrap();
                            *s = TunnelState::Reconnecting;
                        }
                        tokio::time::sleep(Duration::from_millis(options.reconnect_delay_ms)).await;
                        continue;
                    } else {
                        logger.error("Connection failed. Maximum attempts reached or auto-reconnect disabled.");
                        {
                            let mut s = state.write().unwrap();
                            *s = TunnelState::Faulted;
                        }
                        break;
                    }
                }

                // Wait for response
                let mut response_buffer = vec![0u8; 1024];
                let response_res = match &mut socket {
                    SocketType::Udp(s) => {
                        match tokio::time::timeout(Duration::from_secs(6), s.recv_from(&mut response_buffer)).await {
                            Ok(Ok((len, _))) => Ok(len),
                            Ok(Err(io_err)) => Err(KnxError::from(io_err)),
                            Err(_) => Err(KnxError::Timeout),
                        }
                    }
                    SocketType::Tcp(rh, _) => {
                        match tokio::time::timeout(Duration::from_secs(6), rh.read(&mut response_buffer)).await {
                            Ok(Ok(0)) => Err(KnxError::ConnectionClosed),
                            Ok(Ok(len)) => Ok(len),
                            Ok(Err(io_err)) => Err(KnxError::from(io_err)),
                            Err(_) => Err(KnxError::Timeout),
                        }
                    }
                };

                let response_len = match response_res {
                    Ok(len) => len,
                    Err(e) => {
                        logger.error(&format!("Error waiting for ConnectResponse: {:?}", e));
                        if let Some(tx) = conn_done_tx.take() {
                            let _ = tx.send(Err(e));
                        }
                        if options.auto_reconnect && attempts < options.max_reconnect_attempts {
                            attempts += 1;
                            logger.info(&format!("Reconnecting attempt {}/{}...", attempts, options.max_reconnect_attempts));
                            {
                                let mut s = state.write().unwrap();
                                *s = TunnelState::Reconnecting;
                            }
                            tokio::time::sleep(Duration::from_millis(options.reconnect_delay_ms)).await;
                            continue;
                        } else {
                            logger.error("Connection failed. Maximum attempts reached or auto-reconnect disabled.");
                            {
                                let mut s = state.write().unwrap();
                                *s = TunnelState::Faulted;
                            }
                            break;
                        }
                    }
                };

                if response_len < 6 {
                    continue;
                }

                let response_header = match KnxNetIpHeader::from_buffer(&response_buffer[..6]) {
                    Ok(h) => h,
                    Err(_) => continue,
                };

                if response_header.service_type == KnxNetIpServiceType::ConnectResponse {
                    let channel = response_buffer[6];
                    let status = response_buffer[7];

                    if status == KnxNetIpErrorCodes::ENoError as u8 {
                        // Success! Parse individual address if present
                        let mut ia_str = "1.0.1".to_string();
                        if response_len >= 16 {
                            if let Ok(crd) = Crd::from_buffer(&response_buffer[16..]) {
                                ia_str = KnxHelper::get_address_to_string(
                                    &[(crd.knx_address >> 8) as u8, (crd.knx_address & 0xFF) as u8],
                                    ".",
                                    false
                                ).unwrap_or_else(|_| "1.0.1".to_string());
                            }
                        }

                        {
                            let mut ia = individual_address.write().unwrap();
                            *ia = ia_str;
                        }

                        {
                            let mut s = state.write().unwrap();
                            *s = TunnelState::Connected;
                        }
                        logger.info("Connected to KNXnet/IP Gateway successfully.");
                        if let Some(tx) = conn_done_tx.take() {
                            let _ = tx.send(Ok(()));
                        }

                        // Run connection loops (heartbeat, message processing, incoming reading)
                        let channel_id = channel;
                        let mut seq_num = 0u8;
                        let mut rx_seq_num = 0u8;

                        let (udp_socket, mut tcp_rh, mut tcp_wh) = match socket {
                            SocketType::Udp(s) => (Some(s), None, None),
                            SocketType::Tcp(rh, wh) => (None, Some(rh), Some(wh)),
                        };

                        let (incoming_packet_tx, mut incoming_packet_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(100);

                        // Spawn reader task
                        let reader_tx = incoming_packet_tx.clone();
                        if let Some(s) = udp_socket.clone() {
                            tokio::spawn(async move {
                                let mut buf = vec![0u8; 1500];
                                loop {
                                    match s.recv_from(&mut buf).await {
                                        Ok((len, addr)) => {
                                            if reader_tx.send((buf[..len].to_vec(), addr)).await.is_err() {
                                                break;
                                            }
                                        }
                                        Err(_) => break,
                                    }
                                }
                            });
                        } else if let Some(mut rh) = tcp_rh.take() {
                            tokio::spawn(async move {
                                let mut buf = vec![0u8; 2048];
                                let mut tcp_buf = Vec::new();
                                loop {
                                    match rh.read(&mut buf).await {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            tcp_buf.extend_from_slice(&buf[..n]);
                                            while tcp_buf.len() >= 6 {
                                                let total_len = ((tcp_buf[4] as usize) << 8) | (tcp_buf[5] as usize);
                                                if tcp_buf.len() >= total_len {
                                                    let pkt = tcp_buf[..total_len].to_vec();
                                                    tcp_buf.drain(..total_len);
                                                    if reader_tx.send((pkt, "0.0.0.0:0".parse().unwrap())).await.is_err() {
                                                        break;
                                                    }
                                                } else {
                                                    break;
                                                }
                                            }
                                        }
                                        Err(_) => break,
                                    }
                                }
                            });
                        }

                        // Helper macro to send packet over UDP or TCP
                        macro_rules! send_raw {
                            ($pkt:expr) => {
                                if let Some(ref s) = udp_socket {
                                    s.send_to($pkt, &host_addr).await.map(|_| ()).map_err(|_| ())
                                } else if let Some(ref mut wh) = tcp_wh {
                                    wh.write_all($pkt).await.map_err(|_| ())
                                } else {
                                    Err(())
                                }
                            };
                        }

                        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(60));
                        heartbeat_interval.tick().await; // Initial tick completes immediately

                        let mut active_send: Option<(oneshot::Sender<Result<(), KnxError>>, Instant, usize)> = None;
                        let mut last_sent_packet = Vec::new();

                        let mut heartbeat_pending = false;
                        let mut last_heartbeat_sent = Instant::now();
                        let mut heartbeat_failures = 0;

                        loop {
                            let ack_timeout = if let Some((_, send_time, _)) = &active_send {
                                let elapsed = send_time.elapsed();
                                let remaining = if elapsed < Duration::from_millis(1000) {
                                    Duration::from_millis(1000) - elapsed
                                } else {
                                    Duration::from_millis(0)
                                };
                                Some(tokio::time::sleep(remaining))
                            } else {
                                None
                            };

                            let heartbeat_timeout = if heartbeat_pending {
                                let elapsed = last_heartbeat_sent.elapsed();
                                let remaining = if elapsed < Duration::from_secs(10) {
                                    Duration::from_secs(10) - elapsed
                                } else {
                                    Duration::from_secs(0)
                                };
                                Some(tokio::time::sleep(remaining))
                            } else {
                                None
                            };

                            tokio::select! {
                                msg = actor_rx.recv(), if active_send.is_none() => {
                                    match msg {
                                        Some(ActorMessage::SendCemi(cemi, reply)) => {
                                            let cemi_bytes = cemi.to_buffer();
                                            let mut conn_header = vec![0x04, channel_id, seq_num, 0x00];
                                            let header = KnxNetIpHeader::new(
                                                KnxNetIpServiceType::TunnellingRequest,
                                                (6 + conn_header.len() + cemi_bytes.len()) as u16,
                                            );
                                            let mut packet = header.to_buffer();
                                            packet.append(&mut conn_header);
                                            packet.extend_from_slice(&cemi_bytes);

                                            last_sent_packet = packet.clone();
                                            let send_res = send_raw!(&packet);

                                            if send_res.is_ok() {
                                                active_send = Some((reply, Instant::now(), 0));
                                            } else {
                                                let _ = reply.send(Err(KnxError::Io("Send raw failed".to_string())));
                                            }
                                        }
                                        Some(ActorMessage::Disconnect) | None => {
                                            break;
                                        }
                                    }
                                }
                                Some((pkt, _addr)) = incoming_packet_rx.recv() => {
                                    if pkt.len() < 6 {
                                        continue;
                                    }
                                    let header = match KnxNetIpHeader::from_buffer(&pkt[..6]) {
                                        Ok(h) => h,
                                        Err(_) => continue,
                                    };
                                    let body = &pkt[KnxNetIpHeader::HEADER_SIZE_10 as usize..];
 
                                    match header.service_type {
                                        KnxNetIpServiceType::TunnellingAck | KnxNetIpServiceType::DeviceConfigurationAck => {
                                            if body.len() >= 4 && body[1] == channel_id && body[2] == seq_num {
                                                let status = body[3];
                                                if status == KnxNetIpErrorCodes::ENoError as u8 {
                                                    if let Some((reply, _, _)) = active_send.take() {
                                                        let _ = reply.send(Ok(()));
                                                    }
                                                    seq_num = seq_num.wrapping_add(1);
                                                } else {
                                                    if let Some((reply, _, _)) = active_send.take() {
                                                        let _ = reply.send(Err(KnxError::Protocol(format!("ACK error status: {}", status))));
                                                    }
                                                    break; // Reconnect
                                                }
                                            }
                                        }
                                        KnxNetIpServiceType::TunnellingRequest | KnxNetIpServiceType::DeviceConfigurationRequest => {
                                            if body.len() >= 4 && body[1] == channel_id {
                                                let req_seq = body[2];
                                                let header_len = body[0] as usize;

                                                if req_seq == rx_seq_num {
                                                    let ack_type = if header.service_type == KnxNetIpServiceType::TunnellingRequest {
                                                        KnxNetIpServiceType::TunnellingAck
                                                    } else {
                                                        KnxNetIpServiceType::DeviceConfigurationAck
                                                    };
                                                    let ack_header = KnxNetIpHeader::new(ack_type, 10);
                                                    let mut ack = ack_header.to_buffer();
                                                    ack.extend_from_slice(&[0x04, channel_id, req_seq, KnxNetIpErrorCodes::ENoError as u8]);
                                                    let _ = send_raw!(&ack);

                                                    rx_seq_num = rx_seq_num.wrapping_add(1);

                                                    if body.len() > header_len {
                                                        let cemi_data = &body[header_len..];
                                                        if let Ok(cemi) = Cemi::from_buffer(cemi_data) {
                                                            let _ = _incoming_tx.send(cemi.clone());
                                                            let _ = GroupAddressCache::get_instance()
                                                                .write()
                                                                .unwrap()
                                                                .process_cemi(&cemi);
                                                        }
                                                    }
                                                } else if req_seq == rx_seq_num.wrapping_sub(1) {
                                                    let ack_type = if header.service_type == KnxNetIpServiceType::TunnellingRequest {
                                                        KnxNetIpServiceType::TunnellingAck
                                                    } else {
                                                        KnxNetIpServiceType::DeviceConfigurationAck
                                                    };
                                                    let ack_header = KnxNetIpHeader::new(ack_type, 10);
                                                    let mut ack = ack_header.to_buffer();
                                                    ack.extend_from_slice(&[0x04, channel_id, req_seq, KnxNetIpErrorCodes::ENoError as u8]);
                                                    let _ = send_raw!(&ack);
                                                }
                                            }
                                        }
                                        KnxNetIpServiceType::ConnectionstateResponse => {
                                            if body.len() >= 2 && body[0] == channel_id {
                                                if body[1] == KnxNetIpErrorCodes::ENoError as u8 {
                                                    heartbeat_pending = false;
                                                    heartbeat_failures = 0;
                                                } else if body[1] == KnxNetIpErrorCodes::EConnectionId as u8 {
                                                    break; // Reconnect
                                                }
                                            }
                                        }
                                        KnxNetIpServiceType::ConnectionstateRequest => {
                                            if body.len() >= 2 && body[0] == channel_id {
                                                let resp_header = KnxNetIpHeader::new(KnxNetIpServiceType::ConnectionstateResponse, 8);
                                                let mut resp = resp_header.to_buffer();
                                                resp.extend_from_slice(&[channel_id, KnxNetIpErrorCodes::ENoError as u8]);
                                                let _ = send_raw!(&resp);
                                            }
                                        }
                                        KnxNetIpServiceType::DisconnectRequest | KnxNetIpServiceType::DisconnectResponse => {
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                                _ = heartbeat_interval.tick() => {
                                    if !heartbeat_pending {
                                        heartbeat_pending = true;
                                        last_heartbeat_sent = Instant::now();

                                        let mut body = vec![channel_id, 0x00];
                                        body.extend_from_slice(&hpai.to_buffer());
                                        let header = KnxNetIpHeader::new(KnxNetIpServiceType::ConnectionstateRequest, (6 + body.len()) as u16);
                                        let mut packet = header.to_buffer();
                                        packet.extend_from_slice(&body);

                                        let _ = send_raw!(&packet);
                                    }
                                }
                                _ = async {
                                    if let Some(t) = ack_timeout {
                                        t.await;
                                        true
                                    } else {
                                        std::future::pending::<()>().await;
                                        false
                                    }
                                } => {
                                    if let Some((reply, _, retry_count)) = active_send.take() {
                                        if retry_count < 1 {
                                            let _ = send_raw!(&last_sent_packet);
                                            active_send = Some((reply, Instant::now(), retry_count + 1));
                                        } else {
                                            let _ = reply.send(Err(KnxError::Timeout));
                                            break;
                                        }
                                    }
                                }
                                _ = async {
                                    if let Some(t) = heartbeat_timeout {
                                        t.await;
                                        true
                                    } else {
                                        std::future::pending::<()>().await;
                                        false
                                    }
                                } => {
                                    heartbeat_failures += 1;
                                    if heartbeat_failures >= 3 {
                                        break; // Reconnect
                                    } else {
                                        last_heartbeat_sent = Instant::now();
                                        let mut body = vec![channel_id, 0x00];
                                        body.extend_from_slice(&hpai.to_buffer());
                                        let header = KnxNetIpHeader::new(KnxNetIpServiceType::ConnectionstateRequest, (6 + body.len()) as u16);
                                        let mut packet = header.to_buffer();
                                        packet.extend_from_slice(&body);
                                        let _ = send_raw!(&packet);
                                    }
                                }
                            }
                        }
                    }
                }

                break;
            }
        });

        conn_done_rx.await.map_err(|_| KnxError::Protocol("Connection task closed unexpectedly".to_string()))?
    }

    async fn disconnect(&self) -> Result<(), KnxError> {
        let mut guard = self.actor_tx.lock().await;
        if let Some(tx) = guard.take() {
            let _ = tx.send(ActorMessage::Disconnect).await;
        }
        {
            let mut s = self.state.write().unwrap();
            *s = TunnelState::Disconnected;
        }
        Ok(())
    }

    async fn send(&self, cemi: &Cemi) -> Result<(), KnxError> {
        if *self.state.read().unwrap() != TunnelState::Connected {
            return Err(KnxError::Protocol("Tunnel client is not connected".to_string()));
        }

        // Process cache
        let _ = crate::core::cache::group_address_cache::GroupAddressCache::get_instance()
            .write()
            .unwrap()
            .process_cemi(cemi);

        let guard = self.actor_tx.lock().await;
        if let Some(tx) = &*guard {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(ActorMessage::SendCemi(cemi.clone(), reply_tx)).await
                .map_err(|_| KnxError::ConnectionClosed)?;
            reply_rx.await.map_err(|_| KnxError::ConnectionClosed)?
        } else {
            Err(KnxError::ConnectionClosed)
        }
    }

    fn connection_state(&self) -> String {
        let s = self.state.read().unwrap();
        match *s {
            TunnelState::Disconnected => "DISCONNECTED".to_string(),
            TunnelState::Connecting => "CONNECTING".to_string(),
            TunnelState::Connected => "CONNECTED".to_string(),
            TunnelState::Reconnecting => "RECONNECTING".to_string(),
            TunnelState::Faulted => "FAULTED".to_string(),
        }
    }

    fn is_connected(&self) -> bool {
        *self.state.read().unwrap() == TunnelState::Connected
    }

    fn individual_address(&self) -> String {
        self.individual_address.read().unwrap().clone()
    }
}
