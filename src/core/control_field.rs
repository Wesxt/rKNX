#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    System = 0, // 00
    Normal = 1, // 01
    Urgent = 2, // 10
    Low = 3,    // 11
}

impl Priority {
    pub fn from_u8(value: u8) -> Self {
        match value & 0x03 {
            0 => Priority::System,
            1 => Priority::Normal,
            2 => Priority::Urgent,
            _ => Priority::Low,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Extended = 0,
    Standard = 1,
}

/// @see <https://my.knx.org/es/shop/knx-specifications?product_type=knx-specifications> - "KNX Standard External Message Interface"
/// @alias Controlfield 1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlField {
    buffer: [u8; 1],
}

impl ControlField {
    pub fn new(initial_value: u8) -> Self {
        Self { buffer: [initial_value] }
    }

    /// Bit 7: Frame Type (FT) - 0: extended, 1: standard
    /// - This shall specify the Frame Type that shall be used for transmission or
    /// reception of the frame.
    pub fn get_frame_type(&self) -> FrameType {
        if (self.buffer[0] & 0x80) == 0x80 {
            FrameType::Standard
        } else {
            FrameType::Extended
        }
    }

    pub fn set_frame_type(&mut self, is_standard: bool) {
        if is_standard {
            self.buffer[0] |= 0x80;
        } else {
            self.buffer[0] &= 0x7F;
        }
    }

    /// Bit 5: Repeat (R) - 0: repeat, 1: do not repeat
    /// - Repeat, not valid for all media
    pub fn get_repeat(&self) -> bool {
        (self.buffer[0] & 0x20) == 0x20
    }

    pub fn set_repeat(&mut self, do_not_repeat: bool) {
        if do_not_repeat {
            self.buffer[0] |= 0x20;
        } else {
            self.buffer[0] &= 0xDF;
        }
    }

    /// Bit 4: System Broadcast (SB) - 0: system broadcast, 1: broadcast
    /// - This shall specify whether the frame is transmitted using system broadcast
    /// communication mode or broadcast communication mode (applicable only on
    /// open media);
    pub fn get_system_broadcast(&self) -> bool {
        (self.buffer[0] & 0x10) == 0x10
    }

    pub fn set_system_broadcast(&mut self, is_broadcast: bool) {
        if is_broadcast {
            self.buffer[0] |= 0x10;
        } else {
            self.buffer[0] &= 0xEF;
        }
    }

    /// Bits 3 and 2: Priority (P)
    /// - This shall specify that Priority that shall be used for transmission or
    /// reception of the frame.
    pub fn get_priority(&self) -> Priority {
        Priority::from_u8((self.buffer[0] >> 2) & 0x03)
    }

    pub fn set_priority(&mut self, priority: Priority) {
        self.buffer[0] = (self.buffer[0] & 0xF3) | ((priority as u8 & 0x03) << 2);
    }

    /// Bit 1: Acknowledge request (A) - 0: no ack, 1: ack requested
    /// - This shall specify whether a L2-acknowledge shall be requested for the
    /// L_Data.req frame or not. This is not valid for all media.
    pub fn get_ack_request(&self) -> bool {
        (self.buffer[0] & 0x02) == 0x02
    }

    pub fn set_ack_request(&mut self, requested: bool) {
        if requested {
            self.buffer[0] |= 0x02;
        } else {
            self.buffer[0] &= 0xFD;
        }
    }

    /// Bit 0: Confirm (C) - 0: no error, 1: error
    /// - This shall specify whether a L2-acknowledge shall be requested for the
    /// L_Data.req frame or not. This is not valid for all media.
    pub fn get_confirm(&self) -> bool {
        (self.buffer[0] & 0x01) == 0x01
    }

    pub fn set_confirm(&mut self, has_error: bool) {
        if has_error {
            self.buffer[0] |= 0x01;
        } else {
            self.buffer[0] &= 0xfe;
        }
    }

    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn describe(&self) -> ControlFieldDescription {
        ControlFieldDescription {
            obj: "ControlField",
            hex: format!("0x{:02X}", self.buffer[0]),
            frame_type: match self.get_frame_type() {
                FrameType::Standard => "STANDARD",
                FrameType::Extended => "EXTENDED",
            },
            repeat: self.get_repeat(),
            system_broadcast: self.get_system_broadcast(),
            priority: match self.get_priority() {
                Priority::System => "SYSTEM",
                Priority::Normal => "NORMAL",
                Priority::Urgent => "URGENT",
                Priority::Low => "LOW",
            },
            ack_request: if self.get_ack_request() {
                "acknowledge requested"
            } else {
                "no acknowledge is requested"
            },
            confirm: if self.get_confirm() {
                "error"
            } else {
                "no error"
            },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ControlFieldDescription {
    pub obj: &'static str,
    pub hex: String,
    pub frame_type: &'static str,
    pub repeat: bool,
    pub system_broadcast: bool,
    pub priority: &'static str,
    pub ack_request: &'static str,
    pub confirm: &'static str,
}
