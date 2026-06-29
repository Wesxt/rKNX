#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnxNetIpServiceType {
    SearchRequest = 0x0201,
    SearchResponse = 0x0202,
    DescriptionRequest = 0x0203,
    DescriptionResponse = 0x0204,
    ConnectRequest = 0x0205,
    ConnectResponse = 0x0206,
    ConnectionstateRequest = 0x0207,
    ConnectionstateResponse = 0x0208,
    DisconnectRequest = 0x0209,
    DisconnectResponse = 0x020A,
    SearchRequestExtended = 0x020B,
    SearchResponseExtended = 0x020C,
    DeviceConfigurationRequest = 0x0310,
    DeviceConfigurationAck = 0x0311,
    TunnellingRequest = 0x0420,
    TunnellingAck = 0x0421,
    TunnellingFeatureGet = 0x0422,
    TunnellingFeatureResponse = 0x0423,
    TunnellingFeatureSet = 0x0424,
    TunnellingFeatureInfo = 0x0425,
    RoutingIndication = 0x0530,
    RoutingLostMessage = 0x0531,
    RoutingBusy = 0x0532,
    RoutingSystemBroadcast = 0x0533,
}

impl KnxNetIpServiceType {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0201 => Some(KnxNetIpServiceType::SearchRequest),
            0x0202 => Some(KnxNetIpServiceType::SearchResponse),
            0x0203 => Some(KnxNetIpServiceType::DescriptionRequest),
            0x0204 => Some(KnxNetIpServiceType::DescriptionResponse),
            0x0205 => Some(KnxNetIpServiceType::ConnectRequest),
            0x0206 => Some(KnxNetIpServiceType::ConnectResponse),
            0x0207 => Some(KnxNetIpServiceType::ConnectionstateRequest),
            0x0208 => Some(KnxNetIpServiceType::ConnectionstateResponse),
            0x0209 => Some(KnxNetIpServiceType::DisconnectRequest),
            0x020A => Some(KnxNetIpServiceType::DisconnectResponse),
            0x020B => Some(KnxNetIpServiceType::SearchRequestExtended),
            0x020C => Some(KnxNetIpServiceType::SearchResponseExtended),
            0x0310 => Some(KnxNetIpServiceType::DeviceConfigurationRequest),
            0x0311 => Some(KnxNetIpServiceType::DeviceConfigurationAck),
            0x0420 => Some(KnxNetIpServiceType::TunnellingRequest),
            0x0421 => Some(KnxNetIpServiceType::TunnellingAck),
            0x0422 => Some(KnxNetIpServiceType::TunnellingFeatureGet),
            0x0423 => Some(KnxNetIpServiceType::TunnellingFeatureResponse),
            0x0424 => Some(KnxNetIpServiceType::TunnellingFeatureSet),
            0x0425 => Some(KnxNetIpServiceType::TunnellingFeatureInfo),
            0x0530 => Some(KnxNetIpServiceType::RoutingIndication),
            0x0531 => Some(KnxNetIpServiceType::RoutingLostMessage),
            0x0532 => Some(KnxNetIpServiceType::RoutingBusy),
            0x0533 => Some(KnxNetIpServiceType::RoutingSystemBroadcast),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnxNetIpErrorCodes {
    ENoError = 0x00,
    EHostProtocolType = 0x01,
    EVersionNotSupported = 0x02,
    ESequenceNumber = 0x04,
    EConnectionId = 0x21,
    EConnectionType = 0x22,
    EConnectionOption = 0x23,
    ENoMoreConnections = 0x24,
    ENoMoreUniqueConnections = 0x25,
    EDataConnection = 0x26,
    EKnxConnection = 0x27,
    ETunnellingLayer = 0x29,
    EConnectionInUse = 0x2E,
}

impl KnxNetIpErrorCodes {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(KnxNetIpErrorCodes::ENoError),
            0x01 => Some(KnxNetIpErrorCodes::EHostProtocolType),
            0x02 => Some(KnxNetIpErrorCodes::EVersionNotSupported),
            0x04 => Some(KnxNetIpErrorCodes::ESequenceNumber),
            0x21 => Some(KnxNetIpErrorCodes::EConnectionId),
            0x22 => Some(KnxNetIpErrorCodes::EConnectionType),
            0x23 => Some(KnxNetIpErrorCodes::EConnectionOption),
            0x24 => Some(KnxNetIpErrorCodes::ENoMoreConnections),
            0x25 => Some(KnxNetIpErrorCodes::ENoMoreUniqueConnections),
            0x26 => Some(KnxNetIpErrorCodes::EDataConnection),
            0x27 => Some(KnxNetIpErrorCodes::EKnxConnection),
            0x29 => Some(KnxNetIpErrorCodes::ETunnellingLayer),
            0x2E => Some(KnxNetIpErrorCodes::EConnectionInUse),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostProtocolCode {
    Ipv4Udp = 0x01,
    Ipv4Tcp = 0x02,
}

impl HostProtocolCode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(HostProtocolCode::Ipv4Udp),
            0x02 => Some(HostProtocolCode::Ipv4Tcp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    DeviceMgmtConnection = 0x03,
    TunnelConnection = 0x04,
    RemlogConnection = 0x06,
    RemconfConnection = 0x07,
    ObjsvrConnection = 0x08,
}

impl ConnectionType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x03 => Some(ConnectionType::DeviceMgmtConnection),
            0x04 => Some(ConnectionType::TunnelConnection),
            0x06 => Some(ConnectionType::RemlogConnection),
            0x07 => Some(ConnectionType::RemconfConnection),
            0x08 => Some(ConnectionType::ObjsvrConnection),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptionType {
    DeviceInfo = 0x01,
    SuppSvcFamilies = 0x02,
    IpConfig = 0x03,
    IpCurConfig = 0x04,
    KnxAddresses = 0x05,
    TunnellingInfo = 0x07,
    DeviceInfoExtended = 0x08,
    MfrData = 0xFE,
}

impl DescriptionType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(DescriptionType::DeviceInfo),
            0x02 => Some(DescriptionType::SuppSvcFamilies),
            0x03 => Some(DescriptionType::IpConfig),
            0x04 => Some(DescriptionType::IpCurConfig),
            0x05 => Some(DescriptionType::KnxAddresses),
            0x07 => Some(DescriptionType::TunnellingInfo),
            0x08 => Some(DescriptionType::DeviceInfoExtended),
            0xFE => Some(DescriptionType::MfrData),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnxLayer {
    LinkLayer = 0x02,
    RawLayer = 0x04,
    BusmonitorLayer = 0x80,
}

impl KnxLayer {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x02 => Some(KnxLayer::LinkLayer),
            0x04 => Some(KnxLayer::RawLayer),
            0x80 => Some(KnxLayer::BusmonitorLayer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnxMedium {
    Tp1 = 0x02,
    Pl110 = 0x04,
    Rf = 0x10,
    KnxIp = 0x20,
}

impl KnxMedium {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x02 => Some(KnxMedium::Tp1),
            0x04 => Some(KnxMedium::Pl110),
            0x10 => Some(KnxMedium::Rf),
            0x20 => Some(KnxMedium::KnxIp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedSupportedServiceFamilies {
    Core = 0x02,
    DeviceManagement = 0x03,
    Tunnelling = 0x04,
    Routing = 0x05,
    RemoteLogging = 0x06,
    RemoteConfigurationAndDiagnosis = 0x07,
    ObjectServer = 0x08,
    Security = 0x09,
}

impl AllowedSupportedServiceFamilies {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x02 => Some(AllowedSupportedServiceFamilies::Core),
            0x03 => Some(AllowedSupportedServiceFamilies::DeviceManagement),
            0x04 => Some(AllowedSupportedServiceFamilies::Tunnelling),
            0x05 => Some(AllowedSupportedServiceFamilies::Routing),
            0x06 => Some(AllowedSupportedServiceFamilies::RemoteLogging),
            0x07 => Some(AllowedSupportedServiceFamilies::RemoteConfigurationAndDiagnosis),
            0x08 => Some(AllowedSupportedServiceFamilies::ObjectServer),
            0x09 => Some(AllowedSupportedServiceFamilies::Security),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnxTimeoutConstants;

impl KnxTimeoutConstants {
    pub const CONNECT_REQUEST_TIMEOUT: u8 = 10;
    pub const CONNECTIONSTATE_REQUEST_TIMEOUT: u8 = 10;
    pub const DEVICE_CONFIGURATION_REQUEST_TIMEOUT: u8 = 10;
    pub const TUNNELING_REQUEST_TIMEOUT: u8 = 1;
    pub const CONNECTION_ALIVE_TIME: u8 = 120;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelLink {
    TunnelLinklayer = 0x02,
    TunnelRaw = 0x04,
    TunnelBusmonitor = 0x80,
}

impl TunnelLink {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x02 => Some(TunnelLink::TunnelLinklayer),
            0x04 => Some(TunnelLink::TunnelRaw),
            0x80 => Some(TunnelLink::TunnelBusmonitor),
            _ => None,
        }
    }
}
