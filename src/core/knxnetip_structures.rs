use crate::core::device_descriptor_type::DeviceDescriptorType0;
use crate::core::knxnetip_enum::{
    ConnectionType, DescriptionType, HostProtocolCode, KnxMedium, TunnelLink,
};
use crate::errors::KnxError;
use std::net::Ipv4Addr;

// ==========================================
// HPAI (Host Protocol Address Information)
// ==========================================
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hpai {
    pub host_protocol: HostProtocolCode,
    pub ip_address: Ipv4Addr,
    pub port: u16,
}

impl Hpai {
    pub fn new(host_protocol: HostProtocolCode, ip_address: Ipv4Addr, port: u16) -> Self {
        Self {
            host_protocol,
            ip_address,
            port,
        }
    }

    pub fn null_hpai() -> Self {
        Self {
            host_protocol: HostProtocolCode::Ipv4Udp,
            ip_address: Ipv4Addr::new(0, 0, 0, 0),
            port: 0,
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = vec![0u8; 8];
        buffer[0] = 0x08; // Length
        buffer[1] = self.host_protocol as u8;

        let octets = self.ip_address.octets();
        buffer[2..6].copy_from_slice(&octets);

        buffer[6] = (self.port >> 8) as u8;
        buffer[7] = (self.port & 0xFF) as u8;
        buffer
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 8 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let host_proto =
            HostProtocolCode::from_u8(buffer[1]).ok_or(KnxError::InvalidParametersForDpt)?;
        let ip_address = Ipv4Addr::new(buffer[2], buffer[3], buffer[4], buffer[5]);
        let port = ((buffer[6] as u16) << 8) | buffer[7] as u16;

        Ok(Self::new(host_proto, ip_address, port))
    }
}

// ==========================================
// CRI (Connection Request Information)
// ==========================================
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cri {
    pub connection_type: ConnectionType,
    pub knx_layer: u8,
    pub individual_address: Option<u16>,
}

impl Cri {
    pub fn new(
        connection_type: ConnectionType,
        knx_layer: u8,
        individual_address: Option<u16>,
    ) -> Self {
        Self {
            connection_type,
            knx_layer,
            individual_address,
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let len = if self.individual_address.is_some() {
            6
        } else {
            4
        };
        let mut buffer = vec![0u8; len];
        buffer[0] = len as u8;
        buffer[1] = self.connection_type as u8;
        buffer[2] = self.knx_layer;
        buffer[3] = 0; // Unused/Reserved
        if let Some(addr) = self.individual_address {
            buffer[4] = (addr >> 8) as u8;
            buffer[5] = (addr & 0xFF) as u8;
        }
        buffer
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 2 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let len = buffer[0] as usize;
        let connection_type =
            ConnectionType::from_u8(buffer[1]).ok_or(KnxError::InvalidParametersForDpt)?;

        if len == 2 {
            return Ok(Self::new(
                connection_type,
                TunnelLink::TunnelLinklayer as u8,
                None,
            ));
        }

        if buffer.len() < 4 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let knx_layer = buffer[2];
        let mut individual_address = None;

        if len == 6 && buffer.len() >= 6 {
            individual_address = Some(((buffer[4] as u16) << 8) | buffer[5] as u16);
        }

        Ok(Self::new(connection_type, knx_layer, individual_address))
    }
}

// ==========================================
// CRD (Connection Response Data)
// ==========================================
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crd {
    pub connection_type: ConnectionType,
    pub knx_address: u16,
}

impl Crd {
    pub fn new(connection_type: ConnectionType, knx_address: u16) -> Self {
        Self {
            connection_type,
            knx_address,
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = vec![0u8; 4];
        buffer[0] = 0x04;
        buffer[1] = self.connection_type as u8;
        buffer[2] = (self.knx_address >> 8) as u8;
        buffer[3] = (self.knx_address & 0xFF) as u8;
        buffer
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 2 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let len = buffer[0];
        let connection_type =
            ConnectionType::from_u8(buffer[1]).ok_or(KnxError::InvalidParametersForDpt)?;

        if len == 2 {
            return Ok(Self::new(connection_type, 0));
        }

        if buffer.len() < 4 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let knx_address = ((buffer[2] as u16) << 8) | buffer[3] as u16;
        Ok(Self::new(connection_type, knx_address))
    }
}

// ==========================================
// RoutingBusy
// ==========================================
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingBusy {
    pub device_state: u8,
    pub wait_time: u16,
    pub routing_busy_control: u16,
}

impl RoutingBusy {
    pub fn new(device_state: u8, wait_time: u16, routing_busy_control: u16) -> Self {
        Self {
            device_state,
            wait_time,
            routing_busy_control,
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = vec![0u8; 6];
        buffer[0] = 0x04; // Spec length
        buffer[1] = self.device_state;
        buffer[2] = (self.wait_time >> 8) as u8;
        buffer[3] = (self.wait_time & 0xFF) as u8;
        buffer[4] = (self.routing_busy_control >> 8) as u8;
        buffer[5] = (self.routing_busy_control & 0xFF) as u8;
        buffer
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 6 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let device_state = buffer[1];
        let wait_time = ((buffer[2] as u16) << 8) | buffer[3] as u16;
        let routing_busy_control = ((buffer[4] as u16) << 8) | buffer[5] as u16;
        Ok(Self::new(device_state, wait_time, routing_busy_control))
    }
}

// ==========================================
// RoutingLostMessage
// ==========================================
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingLostMessage {
    pub device_state: u8,
    pub lost_messages: u16,
}

impl RoutingLostMessage {
    pub fn new(device_state: u8, lost_messages: u16) -> Self {
        Self {
            device_state,
            lost_messages,
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = vec![0u8; 4];
        buffer[0] = 0x04; // Spec length
        buffer[1] = self.device_state;
        buffer[2] = (self.lost_messages >> 8) as u8;
        buffer[3] = (self.lost_messages & 0xFF) as u8;
        buffer
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 4 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let device_state = buffer[1];
        let lost_messages = ((buffer[2] as u16) << 8) | buffer[3] as u16;
        Ok(Self::new(device_state, lost_messages))
    }
}

// ==========================================
// StatusTunnelingSlot
// ==========================================
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusTunnelingSlot {
    value: u16,
}

impl StatusTunnelingSlot {
    pub fn new(value: u16) -> Self {
        Self { value }
    }

    pub fn default() -> Self {
        Self { value: 0xFFF8 }
    }

    pub fn get_value(&self) -> u16 {
        self.value
    }

    pub fn get_usable(&self) -> bool {
        (self.value & 0x0004) != 0
    }

    pub fn set_usable(&mut self, usable: bool) {
        if usable {
            self.value |= 0x0004;
        } else {
            self.value &= !0x0004;
        }
    }

    pub fn get_authorised(&self) -> bool {
        (self.value & 0x0002) != 0
    }

    pub fn set_authorised(&mut self, auth: bool) {
        if auth {
            self.value |= 0x0002;
        } else {
            self.value &= !0x0002;
        }
    }

    pub fn get_free(&self) -> bool {
        (self.value & 0x0001) != 0
    }

    pub fn set_free(&mut self, free: bool) {
        if free {
            self.value |= 0x0001;
        } else {
            self.value &= !0x0001;
        }
    }
}

// ==========================================
// DIB Structures (Description Information Block)
// ==========================================
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInformationDib {
    pub knx_medium: KnxMedium,
    pub device_status: u8,
    pub individual_address: u16,
    pub project_installation_id: u16,
    pub serial_number: [u8; 6],
    pub routing_multicast_address: Ipv4Addr,
    pub mac_address: [u8; 6],
    pub friendly_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpConfigDib {
    pub ip_address: Ipv4Addr,
    pub subnet_mask: Ipv4Addr,
    pub default_gateway: Ipv4Addr,
    pub ip_capabilities: u8,
    pub ip_assignment_method: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpCurrentConfigDib {
    pub ip_address: Ipv4Addr,
    pub subnet_mask: Ipv4Addr,
    pub default_gateway: Ipv4Addr,
    pub dhcp_server: Ipv4Addr,
    pub ip_assignment_method: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelSlot {
    pub address: u16,
    pub status: StatusTunnelingSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnellingInfoDib {
    pub apdu_length: u16,
    pub slots: Vec<TunnelSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedDeviceInformationDib {
    pub medium_status: bool,
    pub maximal_local_apdu_length: u16,
    pub device_descriptor_type0: DeviceDescriptorType0,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedService {
    pub family: u8,
    pub version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedServicesDib {
    pub services: Vec<SupportedService>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnxAddressesDib {
    pub knx_individual_address: u16,
    pub additional_individual_addresses: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MfrDataDib {
    pub manufacturer_id: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownDib {
    pub type_code: u8,
    pub raw_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dib {
    DeviceInfo(DeviceInformationDib),
    IpConfig(IpConfigDib),
    IpCurrentConfig(IpCurrentConfigDib),
    TunnellingInfo(TunnellingInfoDib),
    ExtendedDeviceInfo(ExtendedDeviceInformationDib),
    SupportedServices(SupportedServicesDib),
    KnxAddresses(KnxAddressesDib),
    MfrData(MfrDataDib),
    Unknown(UnknownDib),
}

impl Dib {
    pub fn get_type(&self) -> DescriptionType {
        match self {
            Dib::DeviceInfo(_) => DescriptionType::DeviceInfo,
            Dib::IpConfig(_) => DescriptionType::IpConfig,
            Dib::IpCurrentConfig(_) => DescriptionType::IpCurConfig,
            Dib::TunnellingInfo(_) => DescriptionType::TunnellingInfo,
            Dib::ExtendedDeviceInfo(_) => DescriptionType::DeviceInfoExtended,
            Dib::SupportedServices(_) => DescriptionType::SuppSvcFamilies,
            Dib::KnxAddresses(_) => DescriptionType::KnxAddresses,
            Dib::MfrData(_) => DescriptionType::MfrData,
            Dib::Unknown(unknown) => {
                DescriptionType::from_u8(unknown.type_code).unwrap_or(DescriptionType::DeviceInfo)
            }
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        match self {
            Dib::DeviceInfo(dib) => {
                let mut buffer = vec![0u8; 54];
                buffer[0] = 54;
                buffer[1] = DescriptionType::DeviceInfo as u8;
                buffer[2] = dib.knx_medium as u8;
                buffer[3] = dib.device_status;
                buffer[4] = (dib.individual_address >> 8) as u8;
                buffer[5] = (dib.individual_address & 0xFF) as u8;
                buffer[6] = (dib.project_installation_id >> 8) as u8;
                buffer[7] = (dib.project_installation_id & 0xFF) as u8;
                buffer[8..14].copy_from_slice(&dib.serial_number);
                buffer[14..18].copy_from_slice(&dib.routing_multicast_address.octets());
                buffer[18..24].copy_from_slice(&dib.mac_address);

                let friendly_bytes = dib.friendly_name.as_bytes();
                let limit = friendly_bytes.len().min(30);
                buffer[24..24 + limit].copy_from_slice(&friendly_bytes[..limit]);
                buffer
            }
            Dib::IpConfig(dib) => {
                let mut buffer = vec![0u8; 16];
                buffer[0] = 16;
                buffer[1] = DescriptionType::IpConfig as u8;
                buffer[2..6].copy_from_slice(&dib.ip_address.octets());
                buffer[6..10].copy_from_slice(&dib.subnet_mask.octets());
                buffer[10..14].copy_from_slice(&dib.default_gateway.octets());
                buffer[14] = dib.ip_capabilities;
                buffer[15] = dib.ip_assignment_method;
                buffer
            }
            Dib::IpCurrentConfig(dib) => {
                let mut buffer = vec![0u8; 20];
                buffer[0] = 20;
                buffer[1] = DescriptionType::IpCurConfig as u8;
                buffer[2..6].copy_from_slice(&dib.ip_address.octets());
                buffer[6..10].copy_from_slice(&dib.subnet_mask.octets());
                buffer[10..14].copy_from_slice(&dib.default_gateway.octets());
                buffer[14..18].copy_from_slice(&dib.dhcp_server.octets());
                buffer[18] = dib.ip_assignment_method;
                buffer[19] = 0; // Reserved
                buffer
            }
            Dib::TunnellingInfo(dib) => {
                let len = 4 + dib.slots.len() * 4;
                let mut buffer = vec![0u8; len];
                buffer[0] = len as u8;
                buffer[1] = DescriptionType::TunnellingInfo as u8;
                buffer[2] = (dib.apdu_length >> 8) as u8;
                buffer[3] = (dib.apdu_length & 0xFF) as u8;
                let mut offset = 4;
                for slot in &dib.slots {
                    buffer[offset] = (slot.address >> 8) as u8;
                    buffer[offset + 1] = (slot.address & 0xFF) as u8;
                    buffer[offset + 2] = (slot.status.value >> 8) as u8;
                    buffer[offset + 3] = (slot.status.value & 0xFF) as u8;
                    offset += 4;
                }
                buffer
            }
            Dib::ExtendedDeviceInfo(dib) => {
                let mut buffer = vec![0u8; 8];
                buffer[0] = 8;
                buffer[1] = DescriptionType::DeviceInfoExtended as u8;
                buffer[2] = if dib.medium_status { 1 } else { 0 };
                buffer[3] = 0; // Reserved
                buffer[4] = (dib.maximal_local_apdu_length >> 8) as u8;
                buffer[5] = (dib.maximal_local_apdu_length & 0xFF) as u8;
                buffer[6] = (dib.device_descriptor_type0.value() >> 8) as u8;
                buffer[7] = (dib.device_descriptor_type0.value() & 0xFF) as u8;
                buffer
            }
            Dib::SupportedServices(dib) => {
                let len = 2 + dib.services.len() * 2;
                let mut buffer = vec![0u8; len];
                buffer[0] = len as u8;
                buffer[1] = DescriptionType::SuppSvcFamilies as u8;
                let mut offset = 2;
                for svc in &dib.services {
                    buffer[offset] = svc.family;
                    buffer[offset + 1] = svc.version;
                    offset += 2;
                }
                buffer
            }
            Dib::KnxAddresses(dib) => {
                let len = 4 + dib.additional_individual_addresses.len() * 2;
                let mut buffer = vec![0u8; len];
                buffer[0] = len as u8;
                buffer[1] = DescriptionType::KnxAddresses as u8;
                buffer[2] = (dib.knx_individual_address >> 8) as u8;
                buffer[3] = (dib.knx_individual_address & 0xFF) as u8;
                let mut offset = 4;
                for addr in &dib.additional_individual_addresses {
                    buffer[offset] = (addr >> 8) as u8;
                    buffer[offset + 1] = (addr & 0xFF) as u8;
                    offset += 2;
                }
                buffer
            }
            Dib::MfrData(dib) => {
                let len = 4 + dib.data.len();
                let mut buffer = vec![0u8; len];
                buffer[0] = len as u8;
                buffer[1] = DescriptionType::MfrData as u8;
                buffer[2] = (dib.manufacturer_id >> 8) as u8;
                buffer[3] = (dib.manufacturer_id & 0xFF) as u8;
                buffer[4..].copy_from_slice(&dib.data);
                buffer
            }
            Dib::Unknown(dib) => dib.raw_data.clone(),
        }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 2 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let len = buffer[0] as usize;
        let type_code = buffer[1];

        if buffer.len() < len {
            return Err(KnxError::InvalidParametersForDpt);
        }

        let payload = &buffer[..len];

        match DescriptionType::from_u8(type_code) {
            Some(DescriptionType::DeviceInfo) => {
                if payload.len() < 24 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let knx_medium =
                    KnxMedium::from_u8(payload[2]).ok_or(KnxError::InvalidParametersForDpt)?;
                let device_status = payload[3];
                let individual_address = ((payload[4] as u16) << 8) | payload[5] as u16;
                let project_installation_id = ((payload[6] as u16) << 8) | payload[7] as u16;

                let mut serial_number = [0u8; 6];
                serial_number.copy_from_slice(&payload[8..14]);

                let routing_multicast_address =
                    Ipv4Addr::new(payload[14], payload[15], payload[16], payload[17]);

                let mut mac_address = [0u8; 6];
                mac_address.copy_from_slice(&payload[18..24]);

                let name_buf = &payload[24..];
                let null_byte_idx = name_buf.iter().position(|&x| x == 0);
                let friendly_name = match null_byte_idx {
                    Some(idx) => String::from_utf8_lossy(&name_buf[..idx]).into_owned(),
                    None => String::from_utf8_lossy(name_buf).into_owned(),
                };

                Ok(Dib::DeviceInfo(DeviceInformationDib {
                    knx_medium,
                    device_status,
                    individual_address,
                    project_installation_id,
                    serial_number,
                    routing_multicast_address,
                    mac_address,
                    friendly_name,
                }))
            }
            Some(DescriptionType::IpConfig) => {
                if payload.len() < 16 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let ip_address = Ipv4Addr::new(payload[2], payload[3], payload[4], payload[5]);
                let subnet_mask = Ipv4Addr::new(payload[6], payload[7], payload[8], payload[9]);
                let default_gateway =
                    Ipv4Addr::new(payload[10], payload[11], payload[12], payload[13]);
                let ip_capabilities = payload[14];
                let ip_assignment_method = payload[15];

                Ok(Dib::IpConfig(IpConfigDib {
                    ip_address,
                    subnet_mask,
                    default_gateway,
                    ip_capabilities,
                    ip_assignment_method,
                }))
            }
            Some(DescriptionType::IpCurConfig) => {
                if payload.len() < 20 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let ip_address = Ipv4Addr::new(payload[2], payload[3], payload[4], payload[5]);
                let subnet_mask = Ipv4Addr::new(payload[6], payload[7], payload[8], payload[9]);
                let default_gateway =
                    Ipv4Addr::new(payload[10], payload[11], payload[12], payload[13]);
                let dhcp_server = Ipv4Addr::new(payload[14], payload[15], payload[16], payload[17]);
                let ip_assignment_method = payload[18];

                Ok(Dib::IpCurrentConfig(IpCurrentConfigDib {
                    ip_address,
                    subnet_mask,
                    default_gateway,
                    dhcp_server,
                    ip_assignment_method,
                }))
            }
            Some(DescriptionType::TunnellingInfo) => {
                if payload.len() < 4 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let apdu_length = ((payload[2] as u16) << 8) | payload[3] as u16;
                let mut slots = Vec::new();
                for i in (4..payload.len()).step_by(4) {
                    if i + 3 < payload.len() {
                        let address = ((payload[i] as u16) << 8) | payload[i + 1] as u16;
                        let status_val = ((payload[i + 2] as u16) << 8) | payload[i + 3] as u16;
                        slots.push(TunnelSlot {
                            address,
                            status: StatusTunnelingSlot::new(status_val),
                        });
                    }
                }
                Ok(Dib::TunnellingInfo(TunnellingInfoDib {
                    apdu_length,
                    slots,
                }))
            }
            Some(DescriptionType::DeviceInfoExtended) => {
                if payload.len() < 8 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let medium_status = payload[2] != 0;
                let maximal_local_apdu_length = ((payload[4] as u16) << 8) | payload[5] as u16;
                let desc_val = ((payload[6] as u16) << 8) | payload[7] as u16;

                Ok(Dib::ExtendedDeviceInfo(ExtendedDeviceInformationDib {
                    medium_status,
                    maximal_local_apdu_length,
                    device_descriptor_type0: DeviceDescriptorType0::new(desc_val),
                }))
            }
            Some(DescriptionType::SuppSvcFamilies) => {
                let mut services = Vec::new();
                for i in (2..payload.len()).step_by(2) {
                    if i + 1 < payload.len() {
                        services.push(SupportedService {
                            family: payload[i],
                            version: payload[i + 1],
                        });
                    }
                }
                Ok(Dib::SupportedServices(SupportedServicesDib { services }))
            }
            Some(DescriptionType::KnxAddresses) => {
                if payload.len() < 4 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let knx_individual_address = ((payload[2] as u16) << 8) | payload[3] as u16;
                let mut additional_individual_addresses = Vec::new();
                for i in (4..payload.len()).step_by(2) {
                    if i + 1 < payload.len() {
                        additional_individual_addresses
                            .push(((payload[i] as u16) << 8) | payload[i + 1] as u16);
                    }
                }
                Ok(Dib::KnxAddresses(KnxAddressesDib {
                    knx_individual_address,
                    additional_individual_addresses,
                }))
            }
            Some(DescriptionType::MfrData) => {
                if payload.len() < 4 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let manufacturer_id = ((payload[2] as u16) << 8) | payload[3] as u16;
                let data = payload[4..].to_vec();
                Ok(Dib::MfrData(MfrDataDib {
                    manufacturer_id,
                    data,
                }))
            }
            None => Ok(Dib::Unknown(UnknownDib {
                type_code,
                raw_data: payload.to_vec(),
            })),
        }
    }
}

// ==========================================
// SRP (Service Request Point)
// ==========================================
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Srp {
    pub srp_type: u8,
    pub data: Vec<u8>,
    pub is_mandatory: bool,
}

impl Srp {
    pub fn new(srp_type: u8, data: Vec<u8>, is_mandatory: bool) -> Self {
        Self {
            srp_type,
            data,
            is_mandatory,
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let len = 2 + self.data.length();
        let mut buffer = vec![0u8; len];
        buffer[0] = len as u8;

        let mut type_byte = self.srp_type;
        if self.is_mandatory {
            type_byte |= 0x80;
        }
        buffer[1] = type_byte;
        buffer[2..].copy_from_slice(&self.data);
        buffer
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 2 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let len = buffer[0] as usize;
        if buffer.len() < len {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let type_byte = buffer[1];
        let srp_type = type_byte & 0x7F;
        let is_mandatory = (type_byte & 0x80) != 0;
        let data = buffer[2..len].to_vec();

        Ok(Self::new(srp_type, data, is_mandatory))
    }
}

// A helper extension trait to get length of vector/slice
trait LengthExt {
    fn length(&self) -> usize;
}
impl<T> LengthExt for Vec<T> {
    fn length(&self) -> usize {
        self.len()
    }
}
impl<T> LengthExt for [T] {
    fn length(&self) -> usize {
        self.len()
    }
}
