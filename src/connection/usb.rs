#[cfg(feature = "usb")]
use hidapi::HidApi;

use std::sync::Mutex;
use std::sync::{Arc, RwLock};

use tokio::sync::oneshot;
use tokio::sync::{broadcast, mpsc};
#[cfg(feature = "usb")]
use tokio::time::Duration;

use super::KnxService;
use crate::core::cache::group_address_cache::GroupAddressCache;
use crate::core::cemi::Cemi;
use crate::errors::KnxError;
use crate::utils::logger::Logger;
/// Known KNX USB device vendor IDs.
pub const KNX_USB_VENDOR_IDS: &[u16] = &[
    0x28c2, // Zennio
    0x145c, // ABB / Busch-Jaeger
    0x10a6, // MDT
    0x135e, // Siemens
    0x0e77, // Weinzierl / Siemens
    0x147b, // Weinzierl
    0x16d0, // MCS
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnxUsbState {
    Disconnected,
    Connecting,
    Connected,
    Faulted,
}

/// EMI type supported by the USB device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmiType {
    Emi1 = 0x01,
    Emi2 = 0x02,
    CEmi = 0x03,
}

impl EmiType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0x01 => EmiType::Emi1,
            0x02 => EmiType::Emi2,
            _ => EmiType::CEmi,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KnxUsbOptions {
    pub path: Option<String>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub individual_address: String,
}

#[derive(Debug, Clone)]
pub enum UsbEvent {
    Connected,
    Disconnected,
    Error(String),
    Indication(Cemi),
    RawIndication(Vec<u8>),
    Send(Vec<u8>),
    BusConnected,
    BusDisconnected,
    EmiDiscovery(u8),
    IndicationEmi(Cemi),
}

pub struct KnxUsbConnection {
    options: KnxUsbOptions,
    state: Arc<RwLock<KnxUsbState>>,
    supported_emi_type: Arc<RwLock<EmiType>>,
    bus_connected: Arc<RwLock<bool>>,
    incoming_tx: broadcast::Sender<Cemi>,
    event_tx: broadcast::Sender<UsbEvent>,
    send_tx: mpsc::Sender<Vec<u8>>,
    #[allow(dead_code)]
    send_rx: Arc<tokio::sync::Mutex<Option<mpsc::Receiver<Vec<u8>>>>>,
    logger: Logger,

    #[cfg(feature = "usb")]
    shutdown_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>,
}

impl KnxUsbConnection {
    pub fn new(options: KnxUsbOptions) -> Self {
        let (incoming_tx, _) = broadcast::channel(100);
        let (event_tx, _) = broadcast::channel(100);
        let (send_tx, send_rx) = mpsc::channel(100);
        let logger = Logger::new("KNXUSBConnection");
        Self {
            options,
            state: Arc::new(RwLock::new(KnxUsbState::Disconnected)),
            supported_emi_type: Arc::new(RwLock::new(EmiType::CEmi)),
            bus_connected: Arc::new(RwLock::new(false)),
            incoming_tx,
            event_tx,
            send_tx,
            send_rx: Arc::new(tokio::sync::Mutex::new(Some(send_rx))),
            logger,

            #[cfg(feature = "usb")]
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Cemi> {
        self.incoming_tx.subscribe()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<UsbEvent> {
        self.event_tx.subscribe()
    }

    /// Build a USB transfer frame (8-byte header + body).
    fn build_usb_transfer(protocol_id: u8, emi_id: u8, data: &[u8]) -> Vec<u8> {
        let mut header = vec![0u8; 8];
        header[0] = 0x00; // protocol version
        header[1] = 0x08; // header length
        header[2] = (data.len() >> 8) as u8;
        header[3] = (data.len() & 0xFF) as u8;
        header[4] = protocol_id;
        header[5] = emi_id;
        header[6] = 0x00; // manufacturer code
        header[7] = 0x00;
        header.extend_from_slice(data);
        header
    }

    /// Build a 64-byte HID report from a body payload.
    fn build_hid_report(data: &[u8]) -> Vec<u8> {
        if data.len() > 61 {
            return Vec::new(); // Too long for single packet
        }
        let mut report = vec![0u8; 64];
        report[0] = 0x01; // Report ID
        report[1] = 0x13; // Sequence=1, Type=3 (single-frame packet)
        report[2] = data.len() as u8;
        report[3..3 + data.len()].copy_from_slice(data);
        report
    }

    /// Parse EMI discovery response from bitmask.
    #[allow(dead_code)]
    fn parse_emi_bitmask(bitmask: u8) -> EmiType {
        if bitmask & 0x02 != 0 {
            EmiType::Emi2
        } else if bitmask & 0x01 != 0 {
            EmiType::Emi1
        } else {
            EmiType::CEmi
        }
    }

    /// Parse an incoming HID data frame.
    #[allow(dead_code)]
    fn parse_hid_data(data: &[u8]) -> Option<(u8, u8, Vec<u8>)> {
        if data.len() < 3 || data[0] != 0x01 {
            return None;
        }
        // Only process single-frame packets
        if (data[1] & 0x0F) != 0x03 {
            return None;
        }

        let body_length = data[2] as usize;
        if 3 + body_length > data.len() {
            return None;
        }

        let body = &data[3..3 + body_length];
        if body.len() < 8 {
            return None;
        }

        let header_length = body[1] as usize;
        let payload_length = ((body[2] as usize) << 8) | body[3] as usize;
        let protocol_id = body[4];
        let emi_id = body[5];

        if header_length != 0x08 || body.len() < header_length + payload_length {
            return None;
        }

        let payload = body[header_length..header_length + payload_length].to_vec();
        Some((protocol_id, emi_id, payload))
    }
}

struct UsbReaderContext {
    incoming_tx: broadcast::Sender<Cemi>,
    event_tx: broadcast::Sender<UsbEvent>,
    bus_connected: Arc<RwLock<bool>>,
    supported_emi_type: Arc<RwLock<EmiType>>,
    discovery_tx: Arc<Mutex<Option<oneshot::Sender<EmiType>>>>,
    logger: Logger,
}

impl KnxService for KnxUsbConnection {
    async fn connect(&self) -> Result<(), KnxError> {
        if *self.state.read().unwrap() == KnxUsbState::Connected {
            return Ok(());
        }

        {
            let mut s = self.state.write().unwrap();
            let old = *s;
            *s = KnxUsbState::Connecting;
            self.logger
                .info(&format!("FSM: State transition from {:?} to {:?}", old, *s).to_uppercase());
        }

        self.logger.info("Opening KNX USB device...");

        #[cfg(feature = "usb")]
        {
            let api = HidApi::new().map_err(|e| {
                self.logger
                    .error(&format!("Failed to initialize HidApi: {:?}", e));
                KnxError::Io(e.to_string())
            })?;

            // Resolve and open HID device
            let device = if let Some(ref path) = self.options.path {
                self.logger
                    .info(&format!("Opening KNX USB device at path: {}", path));
                let c_path = std::ffi::CString::new(path.as_str())
                    .map_err(|_| KnxError::InvalidParametersForDpt)?;
                api.open_path(&c_path).map_err(|e| {
                    self.logger
                        .error(&format!("Failed to open device at path: {:?}", e));
                    KnxError::Io(e.to_string())
                })?
            } else {
                let devices = api.device_list();
                let knx_device = devices
                    .filter(|d| {
                        if let (Some(vid), Some(pid)) =
                            (self.options.vendor_id, self.options.product_id)
                        {
                            return d.vendor_id() == vid && d.product_id() == pid;
                        }
                        KNX_USB_VENDOR_IDS.contains(&d.vendor_id())
                            || d.product_string()
                                .map(|p| p.to_lowercase().contains("knx"))
                                .unwrap_or(false)
                    })
                    .next()
                    .ok_or_else(|| {
                        self.logger.error("No KNX USB device found");
                        KnxError::Protocol("No KNX USB device found".to_string())
                    })?;

                self.logger.info(&format!(
                    "Found device: VID=0x{:04x}, PID=0x{:04x}",
                    knx_device.vendor_id(),
                    knx_device.product_id()
                ));

                api.open_path(knx_device.path()).map_err(|e| {
                    self.logger
                        .error(&format!("Failed to open device path: {:?}", e));
                    KnxError::Io(e.to_string())
                })?
            };

            let device_arc = Arc::new(Mutex::new(device));

            let (discovery_tx, discovery_rx) = oneshot::channel::<EmiType>();
            let discovery_sender = Arc::new(Mutex::new(Some(discovery_tx)));

            let (shutdown_tx, _) = broadcast::channel::<()>(1);
            {
                let mut sd_guard = self.shutdown_tx.lock().unwrap();
                *sd_guard = Some(shutdown_tx.clone());
            }

            let reader_ctx = Arc::new(UsbReaderContext {
                incoming_tx: self.incoming_tx.clone(),
                event_tx: self.event_tx.clone(),
                bus_connected: Arc::clone(&self.bus_connected),
                supported_emi_type: Arc::clone(&self.supported_emi_type),
                discovery_tx: Arc::clone(&discovery_sender),
                logger: self.logger.clone(),
            });

            // 1. Spawning Reader Loop Task
            let device_reader = Arc::clone(&device_arc);
            let mut shutdown_rx1 = shutdown_tx.subscribe();
            tokio::spawn(async move {
                let mut buf = [0u8; 64];
                let wanted = [
                    0x01, 0x13, 0x0a, 0x00, 0x08, 0x00, 0x02, 0x0f, 0x04, 0x00, 0x00, 0x03,
                ];

                loop {
                    tokio::select! {
                        _ = shutdown_rx1.recv() => {
                            break;
                        }
                        _ = tokio::time::sleep(Duration::from_millis(5)) => {
                            let read_res = {
                                let lock = device_reader.lock().unwrap();
                                lock.read_timeout(&mut buf, 5)
                            };

                            match read_res {
                                Ok(0) => {}
                                Ok(n) => {
                                    let data = &buf[..n];

                                    // Bus connection state check
                                    if data.len() >= 12 && data[..12] == wanted {
                                        let is_connected_to_bus = (data[12] & 0x01) == 1;
                                        let mut bc = reader_ctx.bus_connected.write().unwrap();
                                        let old_bc = *bc;
                                        *bc = is_connected_to_bus;
                                        if old_bc != is_connected_to_bus {
                                            if is_connected_to_bus {
                                                let _ = reader_ctx.event_tx.send(UsbEvent::BusConnected);
                                            } else {
                                                let _ = reader_ctx.event_tx.send(UsbEvent::BusDisconnected);
                                            }
                                        }
                                        continue;
                                    }

                                    if let Some((protocol_id, emi_id, payload)) = KnxUsbConnection::parse_hid_data(data) {
                                        if protocol_id == 0x0f && emi_id == 0x02 && payload.len() >= 3 && payload[0] == 0x01 {
                                            // Discovery response
                                            let bitmask = payload[2];
                                            let discovered_emi = KnxUsbConnection::parse_emi_bitmask(bitmask);
                                            let _ = reader_ctx.event_tx.send(UsbEvent::EmiDiscovery(discovered_emi as u8));
                                            let mut d_guard = reader_ctx.discovery_tx.lock().unwrap();
                                            if let Some(tx) = d_guard.take() {
                                                let _ = tx.send(discovered_emi);
                                            }
                                            continue;
                                        }

                                        let current_emi_type = *reader_ctx.supported_emi_type.read().unwrap();
                                        if protocol_id == 0x01 && emi_id == (current_emi_type as u8) {
                                            if !payload.is_empty() {
                                                if current_emi_type == EmiType::CEmi {
                                                    if let Ok(cemi) = Cemi::from_buffer(&payload) {
                                                        let _ = reader_ctx.incoming_tx.send(cemi.clone());
                                                        let _ = reader_ctx.event_tx.send(UsbEvent::Indication(cemi.clone()));
                                                        let _ = reader_ctx.event_tx.send(UsbEvent::RawIndication(payload.clone()));
                                                        reader_ctx.logger.log_indication(&cemi);
                                                        reader_ctx.logger.log_indication_raw(&payload);
                                                        let _ = GroupAddressCache::get_instance()
                                                            .write()
                                                            .unwrap()
                                                            .process_cemi(&cemi);
                                                    }
                                                } else {
                                                    if let Ok(cemi) = crate::core::cemi_adapter::CemiAdapter::emi_to_cemi(&payload) {
                                                        let _ = reader_ctx.incoming_tx.send(cemi.clone());
                                                        let _ = reader_ctx.event_tx.send(UsbEvent::Indication(cemi.clone()));
                                                        let _ = reader_ctx.event_tx.send(UsbEvent::RawIndication(payload.clone()));
                                                        let _ = reader_ctx.event_tx.send(UsbEvent::IndicationEmi(cemi.clone()));
                                                        reader_ctx.logger.log_indication(&cemi);
                                                        reader_ctx.logger.log_indication_raw(&payload);
                                                        let _ = GroupAddressCache::get_instance()
                                                            .write()
                                                            .unwrap()
                                                            .process_cemi(&cemi);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    break;
                                }
                            }
                        }
                    }
                }
            });

            // 2. Spawning Writer Loop Task
            let mut send_rx = self.send_rx.lock().await.take().unwrap();
            let device_writer = Arc::clone(&device_arc);
            let mut shutdown_rx2 = shutdown_tx.subscribe();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        Some(report) = send_rx.recv() => {
                            let _ = device_writer.lock().unwrap().write(&report);
                        }
                        _ = shutdown_rx2.recv() => {
                            break;
                        }
                    }
                }
            });

            // 3. Negotiate / Discover EMI type (retrying up to 5 times)
            let mut emi_discovery_rx = Some(discovery_rx);
            let mut negotiated_emi = EmiType::CEmi;

            for _ in 1..=5 {
                let query = KnxUsbConnection::build_usb_transfer(0x0f, 0x01, &[0x01]);
                let report = KnxUsbConnection::build_hid_report(&query);
                {
                    let _ = device_arc.lock().unwrap().write(&report);
                }

                let rx = emi_discovery_rx.take().unwrap();
                match tokio::time::timeout(Duration::from_secs(1), rx).await {
                    Ok(Ok(version)) => {
                        negotiated_emi = version;
                        break;
                    }
                    Ok(Err(_)) => break,
                    Err(_) => {
                        let (new_tx, new_rx) = oneshot::channel::<EmiType>();
                        *discovery_sender.lock().unwrap() = Some(new_tx);
                        emi_discovery_rx = Some(new_rx);
                    }
                }
            }

            {
                let mut emi_guard = self.supported_emi_type.write().unwrap();
                *emi_guard = negotiated_emi;
            }

            // Set active EMI type on device
            let active_cmd =
                KnxUsbConnection::build_usb_transfer(0x0f, 0x03, &[0x05, negotiated_emi as u8]);
            let active_report = KnxUsbConnection::build_hid_report(&active_cmd);
            let _ = device_arc.lock().unwrap().write(&active_report);
            tokio::time::sleep(Duration::from_millis(100)).await;

            if negotiated_emi == EmiType::CEmi {
                // M_RESET_REQ (0xF1)
                let reset_cmd = KnxUsbConnection::build_usb_transfer(0x01, 0x03, &[0xf1]);
                let reset_report = KnxUsbConnection::build_hid_report(&reset_cmd);
                let _ = device_arc.lock().unwrap().write(&reset_report);
                tokio::time::sleep(Duration::from_millis(100)).await;

                // PID_COMM_MODE to DataLinkLayer
                let comm_mode_buf = vec![0xf6, 0x00, 0x08, 0x01, 0x34, 0x10, 0x01, 0x00];
                let comm_cmd = KnxUsbConnection::build_usb_transfer(0x01, 0x03, &comm_mode_buf);
                let comm_report = KnxUsbConnection::build_hid_report(&comm_cmd);
                let _ = device_arc.lock().unwrap().write(&comm_report);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            {
                let mut s = self.state.write().unwrap();
                let old = *s;
                *s = KnxUsbState::Connected;
                self.logger.info(
                    &format!("FSM: State transition from {:?} to {:?}", old, *s).to_uppercase(),
                );
            }

            self.logger
                .info("Connected to KNX USB device successfully.");
            let _ = self.event_tx.send(UsbEvent::Connected);

            return Ok(());
        }

        #[cfg(not(feature = "usb"))]
        {
            Err(KnxError::Protocol("USB feature is not enabled".to_string()))
        }
    }

    async fn disconnect(&self) -> Result<(), KnxError> {
        let mut s = self.state.write().unwrap();
        if *s == KnxUsbState::Disconnected {
            return Ok(());
        }
        let old = *s;
        *s = KnxUsbState::Disconnected;
        self.logger
            .info(&format!("FSM: State transition from {:?} to {:?}", old, *s).to_uppercase());

        self.logger.info("Disconnected from KNX USB device.");
        let _ = self.event_tx.send(UsbEvent::Disconnected);

        {
            let mut bc = self.bus_connected.write().unwrap();
            *bc = false;
        }

        #[cfg(feature = "usb")]
        {
            let mut sd_guard = self.shutdown_tx.lock().unwrap();
            if let Some(tx) = sd_guard.take() {
                let _ = tx.send(());
            }
        }

        Ok(())
    }

    async fn send(&self, cemi: &Cemi) -> Result<(), KnxError> {
        if *self.state.read().unwrap() != KnxUsbState::Connected {
            return Err(KnxError::InvalidParametersForDpt);
        }

        let _ = GroupAddressCache::get_instance()
            .write()
            .unwrap()
            .process_cemi(cemi);

        let frame = cemi.to_buffer();
        let _ = self.event_tx.send(UsbEvent::Send(frame.clone()));

        let emi_frame = {
            let emi_type = *self.supported_emi_type.read().unwrap();
            if emi_type == EmiType::CEmi {
                frame
            } else {
                if let Ok(emi) = crate::core::cemi_adapter::CemiAdapter::cemi_to_emi(cemi) {
                    emi.to_buffer()
                } else {
                    return Err(KnxError::InvalidParametersForDpt);
                }
            }
        };

        let emi_id = *self.supported_emi_type.read().unwrap() as u8;
        let transfer = Self::build_usb_transfer(0x01, emi_id, &emi_frame);
        let report = Self::build_hid_report(&transfer);

        if report.is_empty() {
            return Err(KnxError::InvalidParametersForDpt);
        }

        self.send_tx
            .send(report)
            .await
            .map_err(|_| KnxError::InvalidParametersForDpt)
    }

    fn connection_state(&self) -> String {
        match *self.state.read().unwrap() {
            KnxUsbState::Disconnected => "DISCONNECTED".to_string(),
            KnxUsbState::Connecting => "CONNECTING".to_string(),
            KnxUsbState::Connected => "CONNECTED".to_string(),
            KnxUsbState::Faulted => "FAULTED".to_string(),
        }
    }

    fn is_connected(&self) -> bool {
        *self.state.read().unwrap() == KnxUsbState::Connected
    }

    fn individual_address(&self) -> String {
        self.options.individual_address.clone()
    }
}
