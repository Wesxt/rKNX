use crate::core::knxnetip_enum::KnxNetIpServiceType;
use crate::errors::KnxError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnxNetIpHeader {
    pub service_type: KnxNetIpServiceType,
    pub total_length: u16,
}

impl KnxNetIpHeader {
    pub const HEADER_SIZE_10: u8 = 0x06;
    pub const KNXNETIP_VERSION_10: u8 = 0x10;

    pub fn new(service_type: KnxNetIpServiceType, total_length: u16) -> Self {
        Self {
            service_type,
            total_length,
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = vec![0u8; 6];
        buffer[0] = Self::HEADER_SIZE_10;
        buffer[1] = Self::KNXNETIP_VERSION_10;

        let type_val = self.service_type as u16;
        buffer[2] = (type_val >> 8) as u8;
        buffer[3] = (type_val & 0xFF) as u8;

        buffer[4] = (self.total_length >> 8) as u8;
        buffer[5] = (self.total_length & 0xFF) as u8;

        buffer
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 6 {
            return Err(KnxError::InvalidParametersForDpt);
        }

        let header_size = buffer[0];
        let version = buffer[1];

        if header_size != Self::HEADER_SIZE_10 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        if version != Self::KNXNETIP_VERSION_10 {
            return Err(KnxError::InvalidParametersForDpt);
        }

        let service_type_val = ((buffer[2] as u16) << 8) | buffer[3] as u16;
        let service_type = KnxNetIpServiceType::from_u16(service_type_val)
            .ok_or(KnxError::InvalidParametersForDpt)?;

        let total_length = ((buffer[4] as u16) << 8) | buffer[5] as u16;

        Ok(Self::new(service_type, total_length))
    }
}
