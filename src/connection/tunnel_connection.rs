use std::collections::VecDeque;
use tokio::time::{Duration, Instant};
use crate::core::knxnetip_header::KnxNetIpHeader;
use crate::core::knxnetip_enum::{KnxNetIpServiceType, KnxNetIpErrorCodes, KnxLayer};
use crate::core::knxnetip_structures::Hpai;

/// Action resulting from validating an incoming request's sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestAction {
    /// Expected sequence number — process the frame.
    Process,
    /// Previous sequence number — retransmit the ACK.
    RetransmitAck,
    /// Out of sequence — discard without reply.
    Discard,
}

/// Result of validating a request.
#[derive(Debug, Clone)]
pub struct RequestValidation {
    pub action: RequestAction,
    pub status: u8,
}

/// Pending ACK state for stop-and-wait reliability.
struct PendingAck {
    packet: Vec<u8>,
    seq: u8,
    is_retransmission: bool,
    deadline: Instant,
}

/// Queued outgoing item.
struct QueueItem {
    packet: Vec<u8>,
    seq: u8,
    #[allow(dead_code)]
    service_type: KnxNetIpServiceType,
}

/// Encapsulates a single KNXnet/IP Tunnelling or Management connection state.
///
/// Handles sequence numbers, heartbeats, reliable delivery (stop-and-wait),
/// and retransmissions according to KNX Spec Vol 3/8/4.
///
/// This is a **server-side** per-client connection handler.
/// For connecting **to** a KNX device, use `KnxTunneling` instead.
pub struct TunnelConnection {
    pub channel_id: u8,
    pub sno: u8,
    pub rno: u8,

    pub control_hpai: Hpai,
    pub data_hpai: Hpai,
    pub knx_address: u16,
    pub knx_address_str: String,
    pub knx_layer: KnxLayer,

    // Rate limiting / pacing state
    pub rx_count: u32,
    pub last_rx_time: Instant,

    queue: VecDeque<QueueItem>,
    is_sending: bool,
    pending_ack: Option<PendingAck>,

    heartbeat_deadline: Instant,
    heartbeat_timeout: Duration,
    retransmit_timeout: Duration,
    max_queue_size: usize,
}

impl TunnelConnection {
    pub fn new(
        channel_id: u8,
        control_hpai: Hpai,
        data_hpai: Hpai,
        knx_address: u16,
        knx_address_str: String,
        knx_layer: KnxLayer,
        heartbeat_timeout_ms: u64,
        retransmit_timeout_ms: u64,
        max_queue_size: usize,
    ) -> Self {
        let now = Instant::now();
        Self {
            channel_id,
            sno: 0,
            rno: 0,
            control_hpai,
            data_hpai,
            knx_address,
            knx_address_str,
            knx_layer,
            rx_count: 0,
            last_rx_time: now,
            queue: VecDeque::new(),
            is_sending: false,
            pending_ack: None,
            heartbeat_deadline: now + Duration::from_millis(heartbeat_timeout_ms),
            heartbeat_timeout: Duration::from_millis(heartbeat_timeout_ms),
            retransmit_timeout: Duration::from_millis(retransmit_timeout_ms),
            max_queue_size,
        }
    }

    /// Resets the heartbeat timer. Should be called on any valid activity.
    pub fn reset_heartbeat(&mut self) {
        self.heartbeat_deadline = Instant::now() + self.heartbeat_timeout;
    }

    /// Returns true if the heartbeat has expired.
    pub fn is_heartbeat_expired(&self) -> bool {
        Instant::now() >= self.heartbeat_deadline
    }

    /// Returns true if the pending ACK has timed out.
    pub fn is_ack_timeout(&self) -> bool {
        if let Some(ref ack) = self.pending_ack {
            Instant::now() >= ack.deadline
        } else {
            false
        }
    }

    /// Returns the pending ACK's retransmission status.
    /// Returns `None` if no ACK is pending.
    /// Returns `Some(true)` if it was already a retransmission (second timeout → terminate).
    /// Returns `Some(false)` if it was the first attempt (should retransmit once).
    pub fn pending_ack_is_retransmission(&self) -> Option<bool> {
        self.pending_ack.as_ref().map(|a| a.is_retransmission)
    }

    /// Enqueues a CEMI message to be sent to the client.
    /// Returns `None` if successfully queued, or `Some("queue_full")` if the queue overflows.
    pub fn enqueue(&mut self, cemi_buffer: &[u8], service_type: KnxNetIpServiceType) -> Option<&'static str> {
        if self.max_queue_size > 0 && self.queue.len() >= self.max_queue_size {
            return Some("queue_full");
        }

        let seq = self.sno;
        let tunnel_header = vec![0x04, self.channel_id, seq, 0x00];
        let header = KnxNetIpHeader::new(
            service_type,
            (6 + tunnel_header.len() + cemi_buffer.len()) as u16,
        );
        let mut packet = header.to_buffer();
        packet.extend_from_slice(&tunnel_header);
        packet.extend_from_slice(cemi_buffer);

        self.queue.push_back(QueueItem {
            packet,
            seq,
            service_type,
        });
        self.sno = self.sno.wrapping_add(1);

        None
    }

    /// Processes the outgoing queue. Returns the next packet to send, if any.
    pub fn process_queue(&mut self) -> Option<Vec<u8>> {
        if self.is_sending || self.queue.is_empty() {
            return None;
        }

        self.is_sending = true;
        let item = self.queue.pop_front().unwrap();
        self.send_with_retry(item.packet.clone(), item.seq, false);
        Some(item.packet)
    }

    /// Retransmits the pending packet. Returns the packet to re-send, if applicable.
    pub fn retransmit(&mut self) -> Option<Vec<u8>> {
        if let Some(ref ack) = self.pending_ack {
            if !ack.is_retransmission {
                let packet = ack.packet.clone();
                let seq = ack.seq;
                self.send_with_retry(packet.clone(), seq, true);
                return Some(packet);
            }
        }
        None
    }

    fn send_with_retry(&mut self, packet: Vec<u8>, seq: u8, is_retransmission: bool) {
        self.pending_ack = Some(PendingAck {
            packet,
            seq,
            is_retransmission,
            deadline: Instant::now() + self.retransmit_timeout,
        });
    }

    /// Handles an incoming ACK from the client.
    /// Returns `Ok(())` on success, `Err("ack_error")` if status is non-zero.
    pub fn handle_ack(&mut self, seq: u8, status: u8) -> Result<(), &'static str> {
        self.reset_heartbeat();

        if let Some(ref ack) = self.pending_ack {
            if ack.seq == seq {
                self.pending_ack = None;
                self.is_sending = false;

                if status != KnxNetIpErrorCodes::ENoError as u8 {
                    return Err("ack_error");
                }

                return Ok(());
            }
        }
        // Ignored: ACK for unexpected seq
        Ok(())
    }

    /// Validates an incoming request from the client according to sequence number rules.
    pub fn validate_request(&mut self, seq: u8) -> RequestValidation {
        self.reset_heartbeat();

        if seq == self.rno {
            // Expected sequence number
            self.rno = self.rno.wrapping_add(1);
            RequestValidation {
                action: RequestAction::Process,
                status: KnxNetIpErrorCodes::ENoError as u8,
            }
        } else if seq == self.rno.wrapping_sub(1) {
            // Previous sequence number — retransmit ACK
            RequestValidation {
                action: RequestAction::RetransmitAck,
                status: KnxNetIpErrorCodes::ENoError as u8,
            }
        } else {
            // Out of sequence — discard without reply (Spec 2.6.1)
            RequestValidation {
                action: RequestAction::Discard,
                status: 0,
            }
        }
    }

    /// Returns the destination socket address for sending data to this client.
    pub fn data_endpoint(&self) -> String {
        format!("{}:{}", self.data_hpai.ip_address, self.data_hpai.port)
    }

    /// Returns the control endpoint address.
    pub fn control_endpoint(&self) -> String {
        format!("{}:{}", self.control_hpai.ip_address, self.control_hpai.port)
    }

    /// Closes the connection and cleans up resources.
    pub fn close(&mut self) {
        self.pending_ack = None;
        self.queue.clear();
        self.is_sending = false;
    }

    /// Returns true if there are queued items ready to be processed.
    pub fn has_pending_work(&self) -> bool {
        !self.queue.is_empty() || self.pending_ack.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::knxnetip_structures::Hpai;
    use crate::core::knxnetip_enum::HostProtocolCode;
    use std::net::Ipv4Addr;

    fn make_connection() -> TunnelConnection {
        TunnelConnection::new(
            10,
            Hpai::new(HostProtocolCode::Ipv4Udp, Ipv4Addr::new(192, 168, 1, 100), 3671),
            Hpai::new(HostProtocolCode::Ipv4Udp, Ipv4Addr::new(192, 168, 1, 100), 3672),
            0x1101,
            "1.1.1".to_string(),
            KnxLayer::LinkLayer,
            120_000,
            1_000,
            100,
        )
    }

    #[test]
    fn test_sequence_number_validation() {
        let mut conn = make_connection();
        assert_eq!(conn.rno, 0);

        // Expected seq 0
        let v = conn.validate_request(0);
        assert_eq!(v.action, RequestAction::Process);
        assert_eq!(conn.rno, 1);

        // Expected seq 1
        let v = conn.validate_request(1);
        assert_eq!(v.action, RequestAction::Process);
        assert_eq!(conn.rno, 2);

        // Retransmit: previous seq 1
        let v = conn.validate_request(1);
        assert_eq!(v.action, RequestAction::RetransmitAck);
        assert_eq!(conn.rno, 2); // rno should NOT advance

        // Out of sequence: seq 5
        let v = conn.validate_request(5);
        assert_eq!(v.action, RequestAction::Discard);
    }

    #[test]
    fn test_enqueue_and_process() {
        let mut conn = make_connection();
        let cemi = vec![0x29, 0x00, 0xBC, 0xE0, 0x11, 0x01, 0x09, 0x02, 0x01, 0x00, 0x81];

        assert!(conn.enqueue(&cemi, KnxNetIpServiceType::TunnellingRequest).is_none());
        assert_eq!(conn.sno, 1);

        let packet = conn.process_queue();
        assert!(packet.is_some());
        assert!(conn.is_sending);

        // Second process should return None (still sending)
        let packet2 = conn.process_queue();
        assert!(packet2.is_none());
    }

    #[test]
    fn test_handle_ack_success() {
        let mut conn = make_connection();
        let cemi = vec![0x29, 0x00];
        conn.enqueue(&cemi, KnxNetIpServiceType::TunnellingRequest);
        conn.process_queue();

        // ACK for seq 0
        let result = conn.handle_ack(0, KnxNetIpErrorCodes::ENoError as u8);
        assert!(result.is_ok());
        assert!(!conn.is_sending);
    }

    #[test]
    fn test_handle_ack_error() {
        let mut conn = make_connection();
        let cemi = vec![0x29, 0x00];
        conn.enqueue(&cemi, KnxNetIpServiceType::TunnellingRequest);
        conn.process_queue();

        // ACK with error status
        let result = conn.handle_ack(0, KnxNetIpErrorCodes::ESequenceNumber as u8);
        assert!(result.is_err());
    }

    #[test]
    fn test_queue_overflow() {
        let mut conn = TunnelConnection::new(
            10,
            Hpai::null_hpai(),
            Hpai::null_hpai(),
            0x1101,
            "1.1.1".to_string(),
            KnxLayer::LinkLayer,
            120_000,
            1_000,
            2, // max 2 items
        );

        conn.enqueue(&[0x29], KnxNetIpServiceType::TunnellingRequest);
        conn.enqueue(&[0x29], KnxNetIpServiceType::TunnellingRequest);
        let overflow = conn.enqueue(&[0x29], KnxNetIpServiceType::TunnellingRequest);
        assert_eq!(overflow, Some("queue_full"));
    }

    #[test]
    fn test_wrapping_sequence_numbers() {
        let mut conn = make_connection();
        conn.rno = 255;

        let v = conn.validate_request(255);
        assert_eq!(v.action, RequestAction::Process);
        assert_eq!(conn.rno, 0); // Wraps around to 0

        let v = conn.validate_request(255);
        assert_eq!(v.action, RequestAction::RetransmitAck);
    }
}
