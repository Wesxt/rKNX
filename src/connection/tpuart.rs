#[cfg(feature = "serial")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(feature = "serial")]
use tokio_serial::SerialPortBuilderExt;

#[cfg(feature = "serial")]
use std::sync::Mutex;
use std::sync::{Arc, RwLock};

#[cfg(feature = "serial")]
use tokio::sync::oneshot;
use tokio::sync::{broadcast, mpsc};

#[cfg(feature = "serial")]
use tokio::time::{Duration, Instant};

use super::KnxService;
use crate::core::cache::group_address_cache::GroupAddressCache;
use crate::core::cemi::Cemi;
use crate::errors::KnxError;
#[cfg(feature = "serial")]
use crate::utils::knx_helper::KnxHelper;
use crate::utils::logger::Logger;

pub const UART_SERVICES_RESET_REQ: u8 = 0x01;
pub const UART_SERVICES_RESET_IND: u8 = 0x03;
pub const UART_SERVICES_STATE_REQ: u8 = 0x02;
pub const UART_SERVICES_STATE_IND: u8 = 0x07;
pub const UART_SERVICES_ACTIVATE_BUSMON: u8 = 0x05;
pub const UART_SERVICES_LDATA_CON_POS: u8 = 0x8b;
pub const UART_SERVICES_LDATA_CON_NEG: u8 = 0x0b;
pub const UART_SERVICES_ACK_INFO: u8 = 0x10;
pub const UART_SERVICES_LDATA_START: u8 = 0x80;
pub const UART_SERVICES_LDATA_END: u8 = 0x40;
pub const UART_SERVICES_BUSY: u8 = 0xc0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpuartState {
    Disconnected,
    ResetWait,
    SetAddrWait,
    GetStateWait,
    Online,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub enum ConfirmationEvent {
    PosAck,
    NegAck,
    Busy,
}

#[derive(Debug, Clone)]
pub struct TpuartOptions {
    pub path: String,
    pub ack_group: bool,
    pub ack_individual: bool,
    pub individual_address: String,
}

pub struct TpuartConnection {
    options: TpuartOptions,
    state: Arc<RwLock<TpuartState>>,
    is_busmonitor_mode: Arc<RwLock<bool>>,
    incoming_tx: broadcast::Sender<Cemi>,
    send_tx: mpsc::Sender<Vec<u8>>,
    #[allow(dead_code)]
    send_rx: Arc<tokio::sync::Mutex<Option<mpsc::Receiver<Vec<u8>>>>>,
    logger: Logger,

    // Background task channel handles
    #[cfg(feature = "serial")]
    raw_write_tx: Arc<Mutex<Option<mpsc::Sender<Vec<u8>>>>>,
    #[cfg(feature = "serial")]
    init_tx: Arc<Mutex<Option<oneshot::Sender<Result<(), KnxError>>>>>,
    #[cfg(feature = "serial")]
    last_sent_frame: Arc<RwLock<Option<Vec<u8>>>>,
    #[cfg(feature = "serial")]
    shutdown_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>,
}

impl TpuartConnection {
    pub fn new(options: TpuartOptions) -> Self {
        let (incoming_tx, _) = broadcast::channel(100);
        let (send_tx, send_rx) = mpsc::channel(100);
        let logger = Logger::new("TPUART");
        Self {
            options,
            state: Arc::new(RwLock::new(TpuartState::Disconnected)),
            is_busmonitor_mode: Arc::new(RwLock::new(false)),
            incoming_tx,
            send_tx,
            send_rx: Arc::new(tokio::sync::Mutex::new(Some(send_rx))),
            logger,

            #[cfg(feature = "serial")]
            raw_write_tx: Arc::new(Mutex::new(None)),
            #[cfg(feature = "serial")]
            init_tx: Arc::new(Mutex::new(None)),
            #[cfg(feature = "serial")]
            last_sent_frame: Arc::new(RwLock::new(None)),
            #[cfg(feature = "serial")]
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Cemi> {
        self.incoming_tx.subscribe()
    }

    /// Helper for checking if a received raw frame is an echo of the last sent frame.
    /// knxd pattern: ignore repeat bit 0x20
    pub fn is_echo(last_sent: &[u8], received: &[u8]) -> bool {
        if last_sent.len() != received.len() || last_sent.is_empty() {
            return false;
        }
        if (received[0] & !0x20) == (last_sent[0] & !0x20) {
            if received.len() > 1 {
                return received[1..] == last_sent[1..];
            }
            return true;
        }
        false
    }
}

// Global helper to format telegram to TPUART control/data format
#[allow(dead_code)]
fn to_uart_services(telegram: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(telegram.len() * 2);
    for (i, &byte) in telegram.iter().enumerate() {
        let ctrl = if i == telegram.len() - 1 {
            UART_SERVICES_LDATA_END | ((i as u8) & 0x3F)
        } else {
            UART_SERVICES_LDATA_START | ((i as u8) & 0x3F)
        };
        result.push(ctrl);
        result.push(byte);
    }
    result
}

#[cfg(feature = "serial")]
struct TpuartContext {
    options: TpuartOptions,
    state: Arc<RwLock<TpuartState>>,
    is_busmonitor_mode: Arc<RwLock<bool>>,
    incoming_tx: broadcast::Sender<Cemi>,
    raw_write_tx: mpsc::Sender<Vec<u8>>,
    confirmation_tx: mpsc::Sender<ConfirmationEvent>,
    init_tx: Arc<Mutex<Option<oneshot::Sender<Result<(), KnxError>>>>>,
    last_sent_frame: Arc<RwLock<Option<Vec<u8>>>>,
}

#[cfg(feature = "serial")]
struct TpuartReceiver {
    buffer: Vec<u8>,
    last_read: Instant,
    ext_frame: bool,
}

#[cfg(feature = "serial")]
impl TpuartReceiver {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            last_read: Instant::now(),
            ext_frame: false,
        }
    }

    fn process_byte(&mut self, byte: u8, ctx: &TpuartContext) -> Option<Vec<u8>> {
        let now = Instant::now();

        if self.buffer.is_empty() {
            if is_control_byte(byte) {
                handle_control_byte_static(byte, ctx);
                return None;
            }
            // Support for NCN5120/TPUART2: Ignore frame end (0xCB) and warning frame status (0x13)
            if byte == 0xCB || (byte & 0x17) == 0x13 {
                return None;
            }
        }

        // Inter-byte timeout (1000ms) to reset buffer if sync is lost
        if !self.buffer.is_empty()
            && now.duration_since(self.last_read) > Duration::from_millis(1000)
        {
            self.buffer.clear();
        }

        if self.buffer.is_empty() {
            if is_frame_start(byte) {
                self.ext_frame = (byte & 0x80) == 0;
                self.buffer.push(byte);
                self.last_read = now;
            }
        } else {
            self.buffer.push(byte);
            self.last_read = now;

            let min_len = if self.ext_frame { 7 } else { 6 };
            if self.buffer.len() >= min_len {
                let payload_len = if self.ext_frame {
                    self.buffer[6] as usize
                } else {
                    (self.buffer[5] & 0x0F) as usize
                };
                let total_len = payload_len + if self.ext_frame { 9 } else { 8 };
                if self.buffer.len() >= total_len {
                    let frame = self.buffer[..total_len].to_vec();
                    self.buffer.clear();
                    if validate_checksum(&frame) {
                        return Some(frame);
                    }
                }
            }
        }
        None
    }
}

#[cfg(feature = "serial")]
fn is_control_byte(byte: u8) -> bool {
    byte == UART_SERVICES_RESET_IND
        || byte == UART_SERVICES_LDATA_CON_POS
        || byte == UART_SERVICES_LDATA_CON_NEG
        || byte == UART_SERVICES_BUSY
        || (byte & 0x07) == UART_SERVICES_STATE_IND
}

#[cfg(feature = "serial")]
fn is_frame_start(byte: u8) -> bool {
    (byte & 0x50) == 0x10
}

#[cfg(feature = "serial")]
fn validate_checksum(frame: &[u8]) -> bool {
    let mut checksum = 0u8;
    for &byte in &frame[..frame.len() - 1] {
        checksum ^= byte;
    }
    frame[frame.len() - 1] == (checksum ^ 0xFF)
}

#[cfg(feature = "serial")]
fn handle_control_byte_static(byte: u8, ctx: &TpuartContext) {
    // 1. External ACKs/NACKs from other devices
    if byte == 0xCC || byte == 0x0C || byte == 0xC0 {
        // Log or emit bus_ack if needed
        return;
    }

    if byte == UART_SERVICES_RESET_IND {
        let mut state_guard = ctx.state.write().unwrap();
        if *state_guard == TpuartState::ResetWait {
            *state_guard = TpuartState::SetAddrWait;

            if let Ok(addr_buf) =
                KnxHelper::get_address_from_string(&ctx.options.individual_address)
            {
                let mut set_addr_cmd = vec![0x28];
                set_addr_cmd.extend_from_slice(&addr_buf);
                let _ = ctx.raw_write_tx.try_send(set_addr_cmd);
            }

            *state_guard = TpuartState::GetStateWait;
            let _ = ctx.raw_write_tx.try_send(vec![UART_SERVICES_STATE_REQ]);
        } else if *state_guard == TpuartState::Online {
            *state_guard = TpuartState::ResetWait;
            let _ = ctx.raw_write_tx.try_send(vec![UART_SERVICES_RESET_REQ]);
        }
        return;
    }

    if byte == UART_SERVICES_BUSY {
        let _ = ctx.confirmation_tx.try_send(ConfirmationEvent::Busy);
        return;
    }

    if byte == UART_SERVICES_LDATA_CON_POS {
        let _ = ctx.confirmation_tx.try_send(ConfirmationEvent::PosAck);
        return;
    }

    if byte == UART_SERVICES_LDATA_CON_NEG {
        let _ = ctx.confirmation_tx.try_send(ConfirmationEvent::NegAck);
        return;
    }

    if (byte & 0x07) == UART_SERVICES_STATE_IND {
        let mut state_guard = ctx.state.write().unwrap();
        if *state_guard != TpuartState::Online {
            *state_guard = TpuartState::Online;

            let mut init_guard = ctx.init_tx.lock().unwrap();
            if let Some(tx) = init_guard.take() {
                let _ = tx.send(Ok(()));
            }
        }
    }
}

impl KnxService for TpuartConnection {
    async fn connect(&self) -> Result<(), KnxError> {
        if *self.state.read().unwrap() != TpuartState::Disconnected {
            return Ok(());
        }

        self.logger.info(&format!("Initializing TPUART serial connection at {}...", self.options.path));

        #[cfg(feature = "serial")]
        {
            let port = tokio_serial::new(&self.options.path, 19200)
                .data_bits(tokio_serial::DataBits::Eight)
                .parity(tokio_serial::Parity::Even)
                .stop_bits(tokio_serial::StopBits::One)
                .open_native_async()
                .map_err(|e| {
                    self.logger.error(&format!("Failed to open serial port: {:?}", e));
                    KnxError::Io(e.to_string())
                })?;

            let (mut reader, mut writer) = tokio::io::split(port);
            let (raw_write_tx, mut raw_write_rx) = mpsc::channel::<Vec<u8>>(100);
            let (confirmation_tx, mut confirmation_rx) = mpsc::channel::<ConfirmationEvent>(100);
            let (init_tx, init_rx) = oneshot::channel::<Result<(), KnxError>>();
            let (shutdown_tx, _) = broadcast::channel::<()>(1);

            {
                let mut rw_tx = self.raw_write_tx.lock().unwrap();
                *rw_tx = Some(raw_write_tx.clone());
                let mut init_g = self.init_tx.lock().unwrap();
                *init_g = Some(init_tx);
                let mut sd_tx = self.shutdown_tx.lock().unwrap();
                *sd_tx = Some(shutdown_tx.clone());
            }

            let ctx = Arc::new(TpuartContext {
                options: self.options.clone(),
                state: Arc::clone(&self.state),
                is_busmonitor_mode: Arc::clone(&self.is_busmonitor_mode),
                incoming_tx: self.incoming_tx.clone(),
                raw_write_tx: raw_write_tx.clone(),
                confirmation_tx,
                init_tx: Arc::clone(&self.init_tx),
                last_sent_frame: Arc::clone(&self.last_sent_frame),
            });

            // 1. Raw Serial Write loop
            let mut shutdown_rx1 = shutdown_tx.subscribe();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        Some(data) = raw_write_rx.recv() => {
                            if let Err(_) = writer.write_all(&data).await {
                                break;
                            }
                        }
                        _ = shutdown_rx1.recv() => {
                            break;
                        }
                    }
                }
            });

            // 2. Serial Read loop
            let mut shutdown_rx2 = shutdown_tx.subscribe();
            let ctx_reader = Arc::clone(&ctx);
            tokio::spawn(async move {
                let mut rx_buf = [0u8; 1];
                let mut parser = TpuartReceiver::new();

                loop {
                    tokio::select! {
                        res = reader.read_exact(&mut rx_buf) => {
                            match res {
                                Ok(1) => {
                                    let byte = rx_buf[0];
                                    if let Some(frame) = parser.process_byte(byte, &ctx_reader) {
                                        // Echo cancellation
                                        let is_echo = {
                                            let last = ctx_reader.last_sent_frame.read().unwrap();
                                            if let Some(ref last_bytes) = *last {
                                                TpuartConnection::is_echo(last_bytes, &frame)
                                            } else {
                                                false
                                            }
                                        };

                                        if is_echo {
                                            let mut last = ctx_reader.last_sent_frame.write().unwrap();
                                            *last = None;
                                            continue;
                                        }

                                        // Hardware ACK
                                        if !*ctx_reader.is_busmonitor_mode.read().unwrap() {
                                            let mut ack_byte = UART_SERVICES_ACK_INFO;
                                            if ctx_reader.options.ack_group || ctx_reader.options.ack_individual {
                                                let is_extended = (frame[0] & 0x80) == 0;
                                                let control_byte = if is_extended { frame[1] } else { frame[5] };
                                                let is_group = (control_byte & 0x80) != 0;

                                                if (is_group && ctx_reader.options.ack_group) || (!is_group && ctx_reader.options.ack_individual) {
                                                    ack_byte = 0x11;
                                                }
                                            }
                                            let _ = ctx_reader.raw_write_tx.try_send(vec![ack_byte]);
                                        }

                                        // CEMI parse and dispatch
                                        if *ctx_reader.is_busmonitor_mode.read().unwrap() {
                                            let _ = ctx_reader.incoming_tx.send(Cemi::LBusmonInd(crate::core::cemi::LBusmon {
                                                additional_info: Vec::new(),
                                                data: frame,
                                            }));
                                        } else {
                                            let mut emi_buf = vec![0x29];
                                            emi_buf.extend_from_slice(&frame);
                                            if let Ok(cemi) = crate::core::cemi_adapter::CemiAdapter::emi_to_cemi(&emi_buf) {
                                                let _ = ctx_reader.incoming_tx.send(cemi.clone());
                                                let _ = GroupAddressCache::get_instance()
                                                    .write()
                                                    .unwrap()
                                                    .process_cemi(&cemi);
                                            }
                                        }
                                    }
                                }
                                _ => break,
                            }
                        }
                        _ = shutdown_rx2.recv() => {
                            break;
                        }
                    }
                }
            });

            // 3. Outgoing Queue Writer loop
            let mut shutdown_rx3 = shutdown_tx.subscribe();
            let mut mpsc_rx = self.send_rx.lock().await.take().unwrap();
            let ctx_writer = Arc::clone(&ctx);

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        Some(frame) = mpsc_rx.recv() => {
                            let mut attempts = 0;
                            let max_attempts = 3;

                            // Knxd pattern: Repetition modifies repeat bit
                            let mut telegram = frame.clone();

                            while attempts < max_attempts {
                                if attempts > 0 && !telegram.is_empty() && (telegram[0] & 0x20) != 0 {
                                    telegram[0] &= !0x20;
                                    let last_idx = telegram.len() - 1;
                                    telegram[last_idx] ^= 0x20;
                                }

                                {
                                    let mut last = ctx_writer.last_sent_frame.write().unwrap();
                                    *last = Some(telegram.clone());
                                }

                                let uart_bytes = to_uart_services(&telegram);
                                if let Err(_) = ctx_writer.raw_write_tx.send(uart_bytes).await {
                                    break;
                                }

                                let ack_future = confirmation_rx.recv();
                                let timeout_future = tokio::time::sleep(Duration::from_secs(2));

                                tokio::select! {
                                    event = ack_future => {
                                        match event {
                                            Some(ConfirmationEvent::PosAck) => {
                                                break;
                                            }
                                            Some(ConfirmationEvent::NegAck) => {
                                                attempts += 1;
                                            }
                                            Some(ConfirmationEvent::Busy) => {
                                                attempts += 1;
                                                tokio::time::sleep(Duration::from_millis(50)).await;
                                            }
                                            None => {
                                                break;
                                            }
                                        }
                                    }
                                    _ = timeout_future => {
                                        attempts += 1;
                                    }
                                }
                            }
                        }
                        _ = shutdown_rx3.recv() => {
                            break;
                        }
                    }
                }
            });

            // 4. Initial connection handshake
            {
                let mut state_g = self.state.write().unwrap();
                *state_g = TpuartState::ResetWait;
            }
            let _ = raw_write_tx.send(vec![UART_SERVICES_RESET_REQ]).await;

            // Wait for handshake completion (up to 5s)
            tokio::select! {
                res = init_rx => {
                    match res {
                        Ok(Ok(())) => {
                            // Start keepalive heartbeat loop
                            let mut shutdown_rx4 = shutdown_tx.subscribe();
                            let ctx_ka = Arc::clone(&ctx);
                            tokio::spawn(async move {
                                loop {
                                    tokio::select! {
                                        _ = tokio::time::sleep(Duration::from_secs(10)) => {
                                            let st = *ctx_ka.state.read().unwrap();
                                            if st == TpuartState::Online {
                                                let mut state_g = ctx_ka.state.write().unwrap();
                                                *state_g = TpuartState::GetStateWait;
                                                let _ = ctx_ka.raw_write_tx.try_send(vec![UART_SERVICES_STATE_REQ]);
                                            }
                                        }
                                        _ = shutdown_rx4.recv() => {
                                            break;
                                        }
                                    }
                                }
                            });

                            self.logger.info("TPUART connection initialized successfully.");
                            return Ok(());
                        }
                        _ => {
                            let mut state_g = self.state.write().unwrap();
                            *state_g = TpuartState::Error;
                            self.logger.error("TPUART handshake failed.");
                            return Err(KnxError::Protocol("Handshake failed".to_string()));
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    let mut state_g = self.state.write().unwrap();
                    *state_g = TpuartState::Error;
                    self.logger.error("TPUART handshake timed out after 5 seconds.");
                    return Err(KnxError::Timeout);
                }
            }
        }
        #[cfg(not(feature = "serial"))]
        {
            Err(KnxError::Protocol("Serial feature is not enabled".to_string()))
        }
    }

    async fn disconnect(&self) -> Result<(), KnxError> {
        let mut state_g = self.state.write().unwrap();
        if *state_g == TpuartState::Disconnected {
            return Ok(());
        }
        *state_g = TpuartState::Disconnected;

        self.logger.info("Disconnected from TPUART serial interface.");

        #[cfg(feature = "serial")]
        {
            let mut sd_tx = self.shutdown_tx.lock().unwrap();
            if let Some(tx) = sd_tx.take() {
                let _ = tx.send(());
            }
        }

        Ok(())
    }

    async fn send(&self, cemi: &Cemi) -> Result<(), KnxError> {
        if *self.state.read().unwrap() != TpuartState::Online {
            return Err(KnxError::Protocol("TPUART connection is not online".to_string()));
        }

        let _ = GroupAddressCache::get_instance()
            .write()
            .unwrap()
            .process_cemi(cemi);

        // Convert to EMI for TPUART
        let frame = cemi.to_buffer();

        // Remove 0x29 (L_Data.ind) or 0x11 (L_Data.req) message code if present
        let emi_frame = if !frame.is_empty() {
            frame[1..].to_vec()
        } else {
            frame
        };

        self.send_tx
            .send(emi_frame)
            .await
            .map_err(|_| KnxError::InvalidParametersForDpt)
    }

    fn connection_state(&self) -> String {
        match *self.state.read().unwrap() {
            TpuartState::Disconnected => "DISCONNECTED".to_string(),
            TpuartState::ResetWait => "RESET_WAIT".to_string(),
            TpuartState::SetAddrWait => "SET_ADDR_WAIT".to_string(),
            TpuartState::GetStateWait => "GET_STATE_WAIT".to_string(),
            TpuartState::Online => "ONLINE".to_string(),
            TpuartState::Error => "ERROR".to_string(),
        }
    }

    fn is_connected(&self) -> bool {
        *self.state.read().unwrap() == TpuartState::Online
    }

    fn individual_address(&self) -> String {
        self.options.individual_address.clone()
    }
}

// Enable/disable Busmonitor mode
impl TpuartConnection {
    pub async fn set_busmonitor(&self, enabled: bool) -> Result<(), KnxError> {
        if *self.state.read().unwrap() != TpuartState::Online {
            return Err(KnxError::InvalidParametersForDpt);
        }

        {
            let mut bm = self.is_busmonitor_mode.write().unwrap();
            *bm = enabled;
        }

        #[cfg(feature = "serial")]
        {
            let tx_guard = self.raw_write_tx.lock().unwrap();
            if let Some(ref tx) = *tx_guard {
                if enabled {
                    let _ = tx.send(vec![UART_SERVICES_ACTIVATE_BUSMON]).await;
                } else {
                    // To exit busmonitor, perform a reset
                    let _ = tx.send(vec![UART_SERVICES_RESET_REQ]).await;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_cancellation_logic() {
        // Echo cancellation (knxd pattern: ignore repeat bit 0x20)
        let last_sent = vec![0xBC, 0xE0, 0x11, 0x01, 0x09, 0x02, 0x01, 0x00, 0x81];

        // Exact match
        let exact = vec![0xBC, 0xE0, 0x11, 0x01, 0x09, 0x02, 0x01, 0x00, 0x81];
        assert!(TpuartConnection::is_echo(&last_sent, &exact));

        // Match with repeat bit toggled (0xBC vs 0x9C) -> (0xBC & !0x20) == (0x9C & !0x20) -> 0x9C == 0x9C
        let toggled = vec![0x9C, 0xE0, 0x11, 0x01, 0x09, 0x02, 0x01, 0x00, 0x81];
        assert!(TpuartConnection::is_echo(&last_sent, &toggled));

        // Different data
        let diff = vec![0xBC, 0xE0, 0x11, 0x01, 0x09, 0x02, 0x01, 0x00, 0x82];
        assert!(!TpuartConnection::is_echo(&last_sent, &diff));
    }
}
