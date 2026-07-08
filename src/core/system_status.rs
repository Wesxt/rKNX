use crate::errors::KnxError;

#[derive(Debug, Clone, Default)]
pub struct SystemStatusValues {
    pub prog: bool,
    pub llm: bool,
    pub tle: bool,
    pub ale: bool,
    pub se: bool,
    pub ue: bool,
    pub dm: bool,
    pub parity: bool,
}

pub struct SystemStatus {
    value: u8,
}

impl SystemStatus {
    pub fn new(values: SystemStatusValues) -> Result<Self, KnxError> {
        if values.prog {
            return Err(KnxError::InvalidParametersForDpt);
        }
        if values.dm {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let mut status = Self { value: 0 };
        status.set_llm(values.llm);
        status.set_tle(values.tle);
        status.set_ale(values.ale);
        status.set_se(values.se);
        status.set_ue(values.ue);
        status.set_parity(values.parity);
        Ok(status)
    }

    pub fn get_value(&self) -> u8 {
        self.value
    }

    pub fn set_value(&mut self, new_value: u8) {
        self.value = new_value;
    }

    pub fn get_prog(&self) -> bool {
        false
    }

    pub fn get_dm(&self) -> bool {
        false
    }

    /// enable Busmonitor mode
    /// true = disabled, false = enabled
    /// @param value boolean
    pub fn get_llm(&self) -> bool {
        ((self.value >> 1) & 0x01) == 1
    }

    pub fn set_llm(&mut self, enable: bool) {
        self.value = (self.value & 0xFD) | ((enable as u8) << 1);
    }

    /// enable Transport Layer
    /// true = enabled, false = disabled
    /// @param value boolean
    pub fn get_tle(&self) -> bool {
        ((self.value >> 2) & 0x01) == 1
    }

    pub fn set_tle(&mut self, enable: bool) {
        self.value = (self.value & 0xFB) | ((enable as u8) << 2);
    }

    /// enable Application Layer
    /// true = enabled, false = disabled
    /// @param value boolean
    pub fn get_ale(&self) -> bool {
        ((self.value >> 3) & 0x01) == 1
    }

    pub fn set_ale(&mut self, enable: bool) {
        self.value = (self.value & 0xF7) | ((enable as u8) << 3);
    }

    /// enable PEI
    /// true = enabled, false = disabled
    /// @param value boolean
    pub fn get_se(&self) -> bool {
        ((self.value >> 4) & 0x01) == 1
    }

    pub fn set_se(&mut self, enable: bool) {
        self.value = (self.value & 0xEF) | ((enable as u8) << 4);
    }

    /// enable user program
    /// true = enabled, false = disabled
    /// @param value boolean
    pub fn get_ue(&self) -> bool {
        ((self.value >> 5) & 0x01) == 1
    }

    pub fn set_ue(&mut self, enable: bool) {
        self.value = (self.value & 0xDF) | ((enable as u8) << 5);
    }

    /// true = even parity for the “system status” octet, false = disabled
    /// @param value boolean
    pub fn get_parity(&self) -> bool {
        ((self.value >> 7) & 0x01) == 1
    }

    pub fn set_parity(&mut self, enable: bool) {
        self.value = (self.value & 0x7F) | ((enable as u8) << 7);
    }

    /// Crea una instancia de SystemStatus a partir de un byte
    /// @param byte Byte de estado del sistema
    /// @returns Instancia de SystemStatus
    pub fn from_byte(byte: u8) -> Self {
        Self { value: byte }
    }

    /// Proporciona una descripción legible del estado actual de las propiedades del sistema.
    /// @returns Un objeto que describe el estado de cada propiedad.
    pub fn describe(&self) -> SystemStatusDescription {
        SystemStatusDescription {
            obj: "SystemStatus",
            prog: self.get_prog(),
            llm: if self.get_llm() {
                "disabled"
            } else {
                "enabled"
            },
            tle: self.get_tle(),
            ale: self.get_ale(),
            se: self.get_se(),
            ue: self.get_ue(),
            dm: self.get_dm(),
            parity: if self.get_parity() {
                "even parity"
            } else {
                "disabled"
            },
            value: self.value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemStatusDescription {
    pub obj: &'static str,
    pub prog: bool,
    pub llm: &'static str,
    pub tle: bool,
    pub ale: bool,
    pub se: bool,
    pub ue: bool,
    pub dm: bool,
    pub parity: &'static str,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatusValues {
    pub frame_error: bool,
    pub bit_error: bool,
    pub parity_error: bool,
    pub overflow: bool,
    pub lost: bool,
    pub sequence_number: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    value: u8,
}

impl Status {
    pub fn new(values: StatusValues) -> Result<Self, KnxError> {
        if values.sequence_number > 7 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let mut status = Self { value: 0 };
        status.set_frame_error(values.frame_error);
        status.set_bit_error(values.bit_error);
        status.set_parity_error(values.parity_error);
        status.set_overflow(values.overflow);
        status.set_lost(values.lost);
        status.set_sequence_number(values.sequence_number);
        Ok(status)
    }

    pub fn get_value(&self) -> u8 {
        self.value
    }

    pub fn set_value(&mut self, new_value: u8) {
        self.value = new_value;
    }

    /// A frame error was detected in one or several of the frame bits.
    pub fn get_frame_error(&self) -> bool {
        ((self.value >> 7) & 0x01) == 1
    }

    pub fn set_frame_error(&mut self, val: bool) {
        self.value = (self.value & 0x7F) | ((val as u8) << 7);
    }

    /// An invalid bit is detected in one or several of the frame characters.
    pub fn get_bit_error(&self) -> bool {
        ((self.value >> 6) & 0x01) == 1
    }

    pub fn set_bit_error(&mut self, val: bool) {
        self.value = (self.value & 0xBF) | ((val as u8) << 6);
    }

    /// An invalid parity bit was detected in one or several of the frame bits.
    pub fn get_parity_error(&self) -> bool {
        ((self.value >> 5) & 0x01) == 1
    }

    pub fn set_parity_error(&mut self, val: bool) {
        self.value = (self.value & 0xDF) | ((val as u8) << 5);
    }

    /// The overflow flag is set.
    pub fn get_overflow(&self) -> bool {
        ((self.value >> 4) & 0x01) == 1
    }

    pub fn set_overflow(&mut self, val: bool) {
        self.value = (self.value & 0xEF) | ((val as u8) << 4);
    }

    /// The Lost flag shall be set if at least one frame or frame piece is lost by the Busmonitor.
    pub fn get_lost(&self) -> bool {
        ((self.value >> 3) & 0x01) == 1
    }

    pub fn set_lost(&mut self, val: bool) {
        self.value = (self.value & 0xF7) | ((val as u8) << 3);
    }

    /// Each received frame shall let the Data Link Layer increment the modulo
    /// 8 value of the sequence number. The least significant bit of octet 2 shall
    /// also be the least significant bit of the sequence number.
    pub fn get_sequence_number(&self) -> u8 {
        self.value & 0x07
    }

    pub fn set_sequence_number(&mut self, val: u8) {
        self.value = (self.value & 0xF8) | (val & 0x07);
    }

    /// Crea una instancia de Status a partir de un byte
    /// @param byte Byte de estado
    /// @returns Instancia de Status
    pub fn from_byte(byte: u8) -> Self {
        Self { value: byte }
    }

    pub fn describe(&self) -> StatusDescription {
        StatusDescription {
            obj: "Status",
            frame_error: self.get_frame_error(),
            bit_error: self.get_bit_error(),
            parity_error: self.get_parity_error(),
            overflow: self.get_overflow(),
            lost: self.get_lost(),
            sequence_number: self.get_sequence_number(),
            value: self.value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusDescription {
    pub obj: &'static str,
    pub frame_error: bool,
    pub bit_error: bool,
    pub parity_error: bool,
    pub overflow: bool,
    pub lost: bool,
    pub sequence_number: u8,
    pub value: u8,
}
