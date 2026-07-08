use crate::core::system_status::Status;
use crate::errors::KnxError;
use byteorder::{BigEndian, ByteOrder};

fn parse_data_buffer(
    buffer: &[u8],
    expected_type: u8,
    expected_length: Option<usize>,
) -> Result<&[u8], KnxError> {
    if buffer.len() < 2 {
        return Err(KnxError::InvalidParametersForDpt);
    }
    let type_id = buffer[0];
    let length = buffer[1] as usize;
    if type_id != expected_type {
        return Err(KnxError::InvalidParametersForDpt);
    }
    let data_buffer = &buffer[2..];
    if data_buffer.len() != length {
        return Err(KnxError::InvalidParametersForDpt);
    }
    if let Some(exp_len) = expected_length {
        if length != exp_len {
            return Err(KnxError::InvalidParametersForDpt);
        }
    }
    Ok(data_buffer)
}

// -------------------------------------------------------------------
// 1. PLMediumInfo (0x01)
// -------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct PLMediumInfo {
    pub domain_address: [u8; 2],
}

impl PLMediumInfo {
    pub const TYPE_ID: u8 = 0x01;
    pub const DATA_LENGTH: u8 = 0x02;

    pub fn new(domain_address: [u8; 2]) -> Self {
        Self { domain_address }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        let data = parse_data_buffer(buffer, Self::TYPE_ID, Some(Self::DATA_LENGTH as usize))?;
        let mut domain = [0u8; 2];
        domain.copy_from_slice(&data[0..2]);
        Ok(Self {
            domain_address: domain,
        })
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        vec![
            Self::TYPE_ID,
            Self::DATA_LENGTH,
            self.domain_address[0],
            self.domain_address[1],
        ]
    }

    pub fn describe(&self) -> PLMediumInfoDescription {
        PLMediumInfoDescription {
            obj: "PLMediumInfo",
            type_id: Self::TYPE_ID,
            data_length: Self::DATA_LENGTH,
            domain_address: self.domain_address,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PLMediumInfoDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub domain_address: [u8; 2],
}

// -------------------------------------------------------------------
// 2. RFMediumInformation (0x02)
// -------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct RFMediumInformation {
    rf_info: u8,
    serial_number_or_doa: [u8; 6],
    lfn: u8,
}

impl RFMediumInformation {
    pub const TYPE_ID: u8 = 0x02;
    pub const DATA_LENGTH: u8 = 0x08;

    pub fn new(rf_info: u8, serial_number_or_doa: [u8; 6], lfn: u8) -> Self {
        Self {
            rf_info,
            serial_number_or_doa,
            lfn,
        }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        let data = parse_data_buffer(buffer, Self::TYPE_ID, Some(Self::DATA_LENGTH as usize))?;
        let mut serial = [0u8; 6];
        serial.copy_from_slice(&data[1..7]);
        Ok(Self {
            rf_info: data[0],
            serial_number_or_doa: serial,
            lfn: data[7],
        })
    }

    pub fn get_route_last_flag(&self) -> bool {
        (self.rf_info & 0b10000000) != 0
    }

    pub fn set_route_last_flag(&mut self, value: bool) {
        self.rf_info = if value {
            self.rf_info | 0b10000000
        } else {
            self.rf_info & !0b10000000
        };
    }

    pub fn get_rssi(&self) -> u8 {
        (self.rf_info & 0b00110000) >> 4
    }

    pub fn set_rssi(&mut self, value: u8) {
        self.rf_info = (self.rf_info & !0b00110000) | ((value & 0b11) << 4);
    }

    pub fn get_retransmitter_rssi(&self) -> u8 {
        (self.rf_info & 0b00001100) >> 2
    }

    pub fn set_retransmitter_rssi(&mut self, value: u8) {
        self.rf_info = (self.rf_info & !0b00001100) | ((value & 0b11) << 2);
    }

    pub fn get_battery_state(&self) -> bool {
        (self.rf_info & 0b00000010) != 0
    }

    pub fn set_battery_state(&mut self, value: bool) {
        self.rf_info = if value {
            self.rf_info | 0b00000010
        } else {
            self.rf_info & !0b00000010
        };
    }

    pub fn get_unidir_flag(&self) -> bool {
        (self.rf_info & 0b00000001) != 0
    }

    pub fn set_unidir_flag(&mut self, value: bool) {
        self.rf_info = if value {
            self.rf_info | 0b00000001
        } else {
            self.rf_info & !0b00000001
        };
    }

    pub fn get_serial_number_or_doa(&self) -> &[u8; 6] {
        &self.serial_number_or_doa
    }

    pub fn set_serial_number_or_doa(&mut self, val: [u8; 6]) {
        self.serial_number_or_doa = val;
    }

    pub fn get_lfn(&self) -> u8 {
        self.lfn
    }

    pub fn set_lfn(&mut self, val: u8) {
        self.lfn = val;
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buf = vec![Self::TYPE_ID, Self::DATA_LENGTH, self.rf_info];
        buf.extend_from_slice(&self.serial_number_or_doa);
        buf.push(self.lfn);
        buf
    }

    pub fn describe(&self) -> RFMediumInformationDescription {
        RFMediumInformationDescription {
            obj: "RFMediumInformation",
            type_id: Self::TYPE_ID,
            data_length: Self::DATA_LENGTH,
            rf_info: format!("0x{:02x}", self.rf_info),
            route_last_flag: self.get_route_last_flag(),
            rssi: self.get_rssi(),
            retransmitter_rssi: self.get_retransmitter_rssi(),
            battery_state: if self.get_battery_state() {
                "Battery OK"
            } else {
                "Battery Low"
            },
            unidir_flag: if self.get_unidir_flag() {
                "Unidirectional"
            } else {
                "Bidirectional"
            },
            serial_number_or_doa: self.serial_number_or_doa,
            lfn: self.lfn,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RFMediumInformationDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub rf_info: String,
    pub route_last_flag: bool,
    pub rssi: u8,
    pub retransmitter_rssi: u8,
    pub battery_state: &'static str,
    pub unidir_flag: &'static str,
    pub serial_number_or_doa: [u8; 6],
    pub lfn: u8,
}

// -------------------------------------------------------------------
// 3. ExtendedRelativeTimestamp (0x06)
// -------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct ExtendedRelativeTimestamp {
    pub timestamp: u32,
}

impl ExtendedRelativeTimestamp {
    pub const TYPE_ID: u8 = 0x06;
    pub const DATA_LENGTH: u8 = 0x04;

    pub fn new(timestamp: u32) -> Self {
        Self { timestamp }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        let data = parse_data_buffer(buffer, Self::TYPE_ID, Some(Self::DATA_LENGTH as usize))?;
        let timestamp = BigEndian::read_u32(&data[0..4]);
        Ok(Self { timestamp })
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buf = vec![Self::TYPE_ID, Self::DATA_LENGTH, 0, 0, 0, 0];
        BigEndian::write_u32(&mut buf[2..6], self.timestamp);
        buf
    }

    pub fn describe(&self) -> ExtendedRelativeTimestampDescription {
        ExtendedRelativeTimestampDescription {
            obj: "ExtendedRelativeTimestamp",
            type_id: Self::TYPE_ID,
            data_length: Self::DATA_LENGTH,
            timestamp: self.timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtendedRelativeTimestampDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub timestamp: u32,
}

// -------------------------------------------------------------------
// 4. BiBatInformation (0x07)
// -------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct BiBatInformation {
    bibat_ctrl: u8,
    pub bibat_block: u8,
}

impl BiBatInformation {
    pub const TYPE_ID: u8 = 0x07;
    pub const DATA_LENGTH: u8 = 0x02;

    pub fn new(bibat_ctrl: u8, bibat_block: u8) -> Self {
        Self {
            bibat_ctrl: (bibat_ctrl & 0x0F) << 4,
            bibat_block,
        }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        let data = parse_data_buffer(buffer, Self::TYPE_ID, Some(Self::DATA_LENGTH as usize))?;
        Ok(Self {
            bibat_ctrl: data[0],
            bibat_block: data[1],
        })
    }

    pub fn get_bibat_ctrl(&self) -> u8 {
        (self.bibat_ctrl & 0xF0) >> 4
    }

    pub fn set_bibat_ctrl(&mut self, value: u8) {
        self.bibat_ctrl = (self.bibat_ctrl & 0x0F) | ((value & 0x0F) << 4);
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        vec![
            Self::TYPE_ID,
            Self::DATA_LENGTH,
            self.bibat_ctrl,
            self.bibat_block,
        ]
    }

    pub fn describe(&self) -> BiBatInformationDescription {
        BiBatInformationDescription {
            obj: "BiBatInformation",
            type_id: Self::TYPE_ID,
            data_length: Self::DATA_LENGTH,
            bibat_ctrl: self.get_bibat_ctrl(),
            bibat_block: self.bibat_block,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BiBatInformationDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub bibat_ctrl: u8,
    pub bibat_block: u8,
}

// -------------------------------------------------------------------
// 5. RFMultiInformation (0x08)
// -------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct RFMultiInformation {
    pub transmission_frequency: u8,
    call_channel: u8,
    pub physical_acknowledge: u8,
    pub reception_frequency: u8,
}

impl RFMultiInformation {
    pub const TYPE_ID: u8 = 0x08;
    pub const DATA_LENGTH: u8 = 0x04;

    pub fn new(
        transmission_frequency: u8,
        fast_call_channel: u8,
        slow_call_channel: u8,
        physical_acknowledge: u8,
        reception_frequency: u8,
    ) -> Self {
        Self {
            transmission_frequency,
            call_channel: ((fast_call_channel & 0x0F) << 4) | (slow_call_channel & 0x0F),
            physical_acknowledge,
            reception_frequency,
        }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        let data = parse_data_buffer(buffer, Self::TYPE_ID, Some(Self::DATA_LENGTH as usize))?;
        Ok(Self {
            transmission_frequency: data[0],
            call_channel: data[1],
            physical_acknowledge: data[2],
            reception_frequency: data[3],
        })
    }

    pub fn get_fast_call_channel(&self) -> u8 {
        (self.call_channel & 0xF0) >> 4
    }

    pub fn set_fast_call_channel(&mut self, value: u8) {
        self.call_channel = (self.call_channel & 0x0F) | ((value & 0x0F) << 4);
    }

    pub fn get_slow_call_channel(&self) -> u8 {
        self.call_channel & 0x0F
    }

    pub fn set_slow_call_channel(&mut self, value: u8) {
        self.call_channel = (self.call_channel & 0xF0) | (value & 0x0F);
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        vec![
            Self::TYPE_ID,
            Self::DATA_LENGTH,
            self.transmission_frequency,
            self.call_channel,
            self.physical_acknowledge,
            self.reception_frequency,
        ]
    }

    pub fn describe(&self) -> RFMultiInformationDescription {
        RFMultiInformationDescription {
            obj: "RFMultiInformation",
            type_id: Self::TYPE_ID,
            data_length: Self::DATA_LENGTH,
            transmission_frequency: self.transmission_frequency,
            fast_call_channel: self.get_fast_call_channel(),
            slow_call_channel: self.get_slow_call_channel(),
            physical_acknowledge: self.physical_acknowledge,
            reception_frequency: self.reception_frequency,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RFMultiInformationDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub transmission_frequency: u8,
    pub fast_call_channel: u8,
    pub slow_call_channel: u8,
    pub physical_acknowledge: u8,
    pub reception_frequency: u8,
}

// -------------------------------------------------------------------
// 6. PreambleAndPostamble (0x09)
// -------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct PreambleAndPostamble {
    pub preamble_length: u16,
    pub postamble_length: u8,
}

impl PreambleAndPostamble {
    pub const TYPE_ID: u8 = 0x09;
    pub const DATA_LENGTH: u8 = 0x03;

    pub fn new(preamble_length: u16, postamble_length: u8) -> Self {
        Self {
            preamble_length,
            postamble_length,
        }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        let data = parse_data_buffer(buffer, Self::TYPE_ID, Some(Self::DATA_LENGTH as usize))?;
        let preamble = BigEndian::read_u16(&data[0..2]);
        Ok(Self {
            preamble_length: preamble,
            postamble_length: data[2],
        })
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buf = vec![
            Self::TYPE_ID,
            Self::DATA_LENGTH,
            0,
            0,
            self.postamble_length,
        ];
        BigEndian::write_u16(&mut buf[2..4], self.preamble_length);
        buf
    }

    pub fn describe(&self) -> PreambleAndPostambleDescription {
        PreambleAndPostambleDescription {
            obj: "PreambleAndPostamble",
            type_id: Self::TYPE_ID,
            data_length: Self::DATA_LENGTH,
            preamble_length: self.preamble_length,
            postamble_length: self.postamble_length,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreambleAndPostambleDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub preamble_length: u16,
    pub postamble_length: u8,
}

// -------------------------------------------------------------------
// 7. RFFastACKInformation (0x0A)
// -------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RfFastAck {
    pub status: u8,
    pub info: u8,
}

#[derive(Debug, Clone)]
pub struct RFFastACKInformation {
    fast_acks: Vec<RfFastAck>,
}

impl RFFastACKInformation {
    pub const TYPE_ID: u8 = 0x0A;

    pub fn new(acks: Vec<RfFastAck>) -> Self {
        Self { fast_acks: acks }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 2 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let type_id = buffer[0];
        let length = buffer[1] as usize;
        if type_id != Self::TYPE_ID {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let data_buffer = &buffer[2..];
        if data_buffer.len() != length || length % 2 != 0 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let mut fast_acks = Vec::new();
        for i in (0..length).step_by(2) {
            fast_acks.push(RfFastAck {
                status: data_buffer[i],
                info: data_buffer[i + 1],
            });
        }
        Ok(Self { fast_acks })
    }

    pub fn get_fast_acks(&self) -> Vec<RfFastAck> {
        self.fast_acks.clone()
    }

    pub fn set_fast_acks(&mut self, acks: Vec<RfFastAck>) {
        self.fast_acks = acks;
    }

    pub fn add_fast_ack(&mut self, ack: RfFastAck) {
        self.fast_acks.push(ack);
    }

    pub fn get_data_length(&self) -> usize {
        self.fast_acks.len() * 2
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        let data_len = self.get_data_length();
        let mut buf = vec![Self::TYPE_ID, data_len as u8];
        for ack in &self.fast_acks {
            buf.push(ack.status);
            buf.push(ack.info);
        }
        buf
    }

    pub fn describe(&self) -> RFFastACKInformationDescription {
        RFFastACKInformationDescription {
            obj: "RFFastACKInformation",
            type_id: Self::TYPE_ID,
            data_length: self.get_data_length() as u8,
            fast_acks: self.get_fast_acks(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RFFastACKInformationDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub fast_acks: Vec<RfFastAck>,
}

// -------------------------------------------------------------------
// 8. ManufacturerSpecificData (0xFE)
// -------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct ManufacturerSpecificData {
    pub manufacturer_id: u16,
    pub subfunction: u8,
    data: Vec<u8>,
}

impl ManufacturerSpecificData {
    pub const TYPE_ID: u8 = 0xFE;
    pub const MIN_DATA_LENGTH: u8 = 3;

    pub fn new(manufacturer_id: u16, subfunction: u8, data: Vec<u8>) -> Self {
        Self {
            manufacturer_id,
            subfunction,
            data,
        }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 2 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let type_id = buffer[0];
        let length = buffer[1] as usize;
        if type_id != Self::TYPE_ID {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let data_buffer = &buffer[2..];
        if data_buffer.len() != length || length < Self::MIN_DATA_LENGTH as usize {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let m_id = BigEndian::read_u16(&data_buffer[0..2]);
        let sub = data_buffer[2];
        let d = data_buffer[3..].to_vec();
        Ok(Self {
            manufacturer_id: m_id,
            subfunction: sub,
            data: d,
        })
    }

    pub fn get_data(&self) -> &[u8] {
        &self.data
    }

    pub fn set_data(&mut self, val: Vec<u8>) {
        self.data = val;
    }

    pub fn get_data_length(&self) -> usize {
        3 + self.data.len()
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        let data_len = self.get_data_length();
        let mut buf = vec![Self::TYPE_ID, data_len as u8, 0, 0, self.subfunction];
        BigEndian::write_u16(&mut buf[2..4], self.manufacturer_id);
        buf.extend_from_slice(&self.data);
        buf
    }

    pub fn describe(&self) -> ManufacturerSpecificDataDescription {
        ManufacturerSpecificDataDescription {
            obj: "ManufacturerSpecificData",
            type_id: Self::TYPE_ID,
            data_length: self.get_data_length() as u8,
            manufacturer_id: self.manufacturer_id,
            subfunction: self.subfunction,
            data: self.data.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ManufacturerSpecificDataDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub manufacturer_id: u16,
    pub subfunction: u8,
    pub data: Vec<u8>,
}

// -------------------------------------------------------------------
// 9. BusmonitorStatusInfo (0x03)
// -------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusmonitorStatusInfo {
    pub status: Status,
}

impl BusmonitorStatusInfo {
    pub const TYPE_ID: u8 = 0x03;
    pub const DATA_LENGTH: u8 = 0x01;

    pub fn new(status: Status) -> Self {
        Self { status }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        let data = parse_data_buffer(buffer, Self::TYPE_ID, Some(Self::DATA_LENGTH as usize))?;
        Ok(Self {
            status: Status::from_byte(data[0]),
        })
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        vec![Self::TYPE_ID, Self::DATA_LENGTH, self.status.get_value()]
    }

    pub fn describe(&self) -> BusmonitorStatusInfoDescription {
        BusmonitorStatusInfoDescription {
            obj: "BusmonitorStatusInfo",
            type_id: Self::TYPE_ID,
            data_length: Self::DATA_LENGTH,
            status: self.status.describe(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BusmonitorStatusInfoDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub status: crate::core::system_status::StatusDescription,
}

// -------------------------------------------------------------------
// 10. TimestampRelative (0x04)
// -------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct TimestampRelative {
    pub timestamp: u16,
}

impl TimestampRelative {
    pub const TYPE_ID: u8 = 0x04;
    pub const DATA_LENGTH: u8 = 0x02;

    pub fn new(timestamp: u16) -> Self {
        Self { timestamp }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        let data = parse_data_buffer(buffer, Self::TYPE_ID, Some(Self::DATA_LENGTH as usize))?;
        let timestamp = BigEndian::read_u16(&data[0..2]);
        Ok(Self { timestamp })
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buf = vec![Self::TYPE_ID, Self::DATA_LENGTH, 0, 0];
        BigEndian::write_u16(&mut buf[2..4], self.timestamp);
        buf
    }

    pub fn describe(&self) -> TimestampRelativeDescription {
        TimestampRelativeDescription {
            obj: "TimestampRelative",
            type_id: Self::TYPE_ID,
            data_length: Self::DATA_LENGTH,
            timestamp: self.timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimestampRelativeDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub timestamp: u16,
}

// -------------------------------------------------------------------
// 11. TimeDelayUntilSending (0x05)
// -------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct TimeDelayUntilSending {
    pub delay: u16,
}

impl TimeDelayUntilSending {
    pub const TYPE_ID: u8 = 0x05;
    pub const DATA_LENGTH: u8 = 0x02;

    pub fn new(delay: u16) -> Self {
        Self { delay }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        let data = parse_data_buffer(buffer, Self::TYPE_ID, Some(Self::DATA_LENGTH as usize))?;
        let delay = BigEndian::read_u16(&data[0..2]);
        Ok(Self { delay })
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buf = vec![Self::TYPE_ID, Self::DATA_LENGTH, 0, 0];
        BigEndian::write_u16(&mut buf[2..4], self.delay);
        buf
    }

    pub fn describe(&self) -> TimeDelayUntilSendingDescription {
        TimeDelayUntilSendingDescription {
            obj: "TimeDelayUntilSending",
            type_id: Self::TYPE_ID,
            data_length: Self::DATA_LENGTH,
            delay: self.delay,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeDelayUntilSendingDescription {
    pub obj: &'static str,
    pub type_id: u8,
    pub data_length: u8,
    pub delay: u16,
}

// -------------------------------------------------------------------
// Unified AddInfo Wrapper Enum
// -------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum AddInfo {
    PLMediumInfo(PLMediumInfo),
    RFMediumInformation(RFMediumInformation),
    BusmonitorStatusInfo(BusmonitorStatusInfo),
    TimestampRelative(TimestampRelative),
    TimeDelayUntilSending(TimeDelayUntilSending),
    ExtendedRelativeTimestamp(ExtendedRelativeTimestamp),
    BiBatInformation(BiBatInformation),
    RFMultiInformation(RFMultiInformation),
    PreambleAndPostamble(PreambleAndPostamble),
    RFFastACKInformation(RFFastACKInformation),
    ManufacturerSpecificData(ManufacturerSpecificData),
}

impl AddInfo {
    pub fn get_type_id(&self) -> u8 {
        match self {
            AddInfo::PLMediumInfo(_) => PLMediumInfo::TYPE_ID,
            AddInfo::RFMediumInformation(_) => RFMediumInformation::TYPE_ID,
            AddInfo::BusmonitorStatusInfo(_) => BusmonitorStatusInfo::TYPE_ID,
            AddInfo::TimestampRelative(_) => TimestampRelative::TYPE_ID,
            AddInfo::TimeDelayUntilSending(_) => TimeDelayUntilSending::TYPE_ID,
            AddInfo::ExtendedRelativeTimestamp(_) => ExtendedRelativeTimestamp::TYPE_ID,
            AddInfo::BiBatInformation(_) => BiBatInformation::TYPE_ID,
            AddInfo::RFMultiInformation(_) => RFMultiInformation::TYPE_ID,
            AddInfo::PreambleAndPostamble(_) => PreambleAndPostamble::TYPE_ID,
            AddInfo::RFFastACKInformation(_) => RFFastACKInformation::TYPE_ID,
            AddInfo::ManufacturerSpecificData(_) => ManufacturerSpecificData::TYPE_ID,
        }
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        match self {
            AddInfo::PLMediumInfo(info) => info.get_buffer(),
            AddInfo::RFMediumInformation(info) => info.get_buffer(),
            AddInfo::BusmonitorStatusInfo(info) => info.get_buffer(),
            AddInfo::TimestampRelative(info) => info.get_buffer(),
            AddInfo::TimeDelayUntilSending(info) => info.get_buffer(),
            AddInfo::ExtendedRelativeTimestamp(info) => info.get_buffer(),
            AddInfo::BiBatInformation(info) => info.get_buffer(),
            AddInfo::RFMultiInformation(info) => info.get_buffer(),
            AddInfo::PreambleAndPostamble(info) => info.get_buffer(),
            AddInfo::RFFastACKInformation(info) => info.get_buffer(),
            AddInfo::ManufacturerSpecificData(info) => info.get_buffer(),
        }
    }

    pub fn total_length(&self) -> usize {
        self.get_buffer().len()
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 2 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let type_id = buffer[0];
        match type_id {
            PLMediumInfo::TYPE_ID => Ok(AddInfo::PLMediumInfo(PLMediumInfo::from_buffer(buffer)?)),
            RFMediumInformation::TYPE_ID => Ok(AddInfo::RFMediumInformation(
                RFMediumInformation::from_buffer(buffer)?,
            )),
            BusmonitorStatusInfo::TYPE_ID => Ok(AddInfo::BusmonitorStatusInfo(
                BusmonitorStatusInfo::from_buffer(buffer)?,
            )),
            TimestampRelative::TYPE_ID => Ok(AddInfo::TimestampRelative(
                TimestampRelative::from_buffer(buffer)?,
            )),
            TimeDelayUntilSending::TYPE_ID => Ok(AddInfo::TimeDelayUntilSending(
                TimeDelayUntilSending::from_buffer(buffer)?,
            )),
            ExtendedRelativeTimestamp::TYPE_ID => Ok(AddInfo::ExtendedRelativeTimestamp(
                ExtendedRelativeTimestamp::from_buffer(buffer)?,
            )),
            BiBatInformation::TYPE_ID => Ok(AddInfo::BiBatInformation(
                BiBatInformation::from_buffer(buffer)?,
            )),
            RFMultiInformation::TYPE_ID => Ok(AddInfo::RFMultiInformation(
                RFMultiInformation::from_buffer(buffer)?,
            )),
            PreambleAndPostamble::TYPE_ID => Ok(AddInfo::PreambleAndPostamble(
                PreambleAndPostamble::from_buffer(buffer)?,
            )),
            RFFastACKInformation::TYPE_ID => Ok(AddInfo::RFFastACKInformation(
                RFFastACKInformation::from_buffer(buffer)?,
            )),
            ManufacturerSpecificData::TYPE_ID => Ok(AddInfo::ManufacturerSpecificData(
                ManufacturerSpecificData::from_buffer(buffer)?,
            )),
            _ => Err(KnxError::InvalidParametersForDpt),
        }
    }
}
