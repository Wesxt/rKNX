#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApciEnum {
    AGroupValueRead = 0x00,
    AGroupValueResponse = 0x40,
    AGroupValueWrite = 0x80,
    AIndividualAddressWrite = 0xC0,
    AIndividualAddressRead = 0x100,
    AIndividualAddressResponse = 0x140,
    AAnalogToDigitalConverterRead = 0x180,
    AAnalogToDigitalConverterResponse = 0x1C0,
    ASystemNetworkParameterRead = 0x1C8,
    ASystemNetworkParameterResponse = 0x1C9,
    ASystemNetworkParameterWrite = 0x1CA,
    PlannedForFutureSystemBroadcastService = 0x1CB,
    APropertyExtValueRead = 0x1CC,
    APropertyExtValueResponse = 0x1CD,
    APropertyExtValueWriteCon = 0x1CE,
    APropertyExtValueWriteConRes = 0x1CF,
    APropertyExtValueWriteUnCon = 0x1D0,
    APropertyExtValueInfoReport = 0x1D1,
    APropertyExtDescriptionRead = 0x1D2,
    APropertyExtDescriptionResponse = 0x1D3,
    AFunctionPropertyExtCommand = 0x1D4,
    AFunctionPropertyExtStateRead = 0x1D5,
    AFunctionPropertyExtStateResponse = 0x1D6,
    AMemoryExtendedWrite = 0x1FB,
    AMemoryExtendedWriteResponse = 0x1FC,
    AMemoryExtendedRead = 0x1FD,
    AMemoryExtendedReadResponse = 0x1FE,
    AMemoryRead = 0x200,
    AMemoryResponse = 0x240,
    AMemoryWrite = 0x280,
    AUserMemoryRead = 0x2C0,
    AUserMemoryResponse = 0x2C1,
    AUserMemoryWrite = 0x2C2,
    AUserMemoryBitWrite = 0x2C4,
    AUserManufacturerInfoRead = 0x2C5,
    AUserManufacturerInfoResponse = 0x2C6,
    AFunctionPropertyCommand = 0x2C7,
    AFunctionPropertyStateRead = 0x2C8,
    AFunctionPropertyStateResponse = 0x2C9,
    ReservedUserMsg = 0x2CA,
    ReservedUserMsg2 = 0x2F7,
    ReservedUserMsg3 = 0x2F8,
    ReservedUserMsg4 = 0x2FE,
    ADeviceDescriptorRead = 0x300,
    ADeviceDescriptorResponse = 0x340,
    ARestart = 0x380,
    AFilterTableOpen = 0x3C0,
    AFilterTableRead = 0x3C1,
    AFilterTableResponse = 0x3C2,
    AFilterTableWrite = 0x3C3,
    ARouterMemoryRead = 0x3C8,
    ARouterMemoryResponse = 0x3C9,
    ARouterMemoryWrite = 0x3CA,
    ARouterStatusRead = 0x3CD,
    ARouterStatusResponse = 0x3CE,
    ARouterStatusWrite = 0x3CF,
    AMemoryBitWrite = 0x3D0,
    AAuthorizeRequest = 0x3D1,
    AAuthorizeResponse = 0x3D2,
    AKeyWrite = 0x3D3,
    AKeyResponse = 0x3D4,
    APropertyValueRead = 0x3D5,
    APropertyValueResponse = 0x3D6,
    APropertyValueWrite = 0x3D7,
    APropertyDescriptionRead = 0x3D8,
    APropertyDescriptionResponse = 0x3D9,
    ANetworkParameterRead = 0x3DA,
    ANetworkParameterResponse = 0x3DB,
    AIndividualAddressSerialNumberRead = 0x3DC,
    AIndividualAddressSerialNumberResponse = 0x3DD,
    AIndividualAddressSerialNumberWrite = 0x3DE,
    Reserved2 = 0x3DF,
    ADomainAddressWrite = 0x3E0,
    ADomainAddressRead = 0x3E1,
    ADomainAddressResponse = 0x3E2,
    ADomainAddressSelectiveRead = 0x3E3,
    ANetworkParameterWrite = 0x3E4,
    ALinkRead = 0x3E5,
    ALinkResponse = 0x3E6,
    ALinkWrite = 0x3E7,
    AGroupPropValueRead = 0x3E8,
    AGroupPropValueResponse = 0x3E9,
    AGroupPropValueWrite = 0x3EA,
    AGroupPropValueInfoReport = 0x3EB,
    ADomainAddressSerialNumberRead = 0x3EC,
    ADomainAddressSerialNumberResponse = 0x3ED,
    ADomainAddressSerialNumberWrite = 0x3EE,
    AFileStreamInfoReport = 0x3F0,
}

impl ApciEnum {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val & 0x03FF {
            0x00 => Some(ApciEnum::AGroupValueRead),
            0x40 => Some(ApciEnum::AGroupValueResponse),
            0x80 => Some(ApciEnum::AGroupValueWrite),
            0xC0 => Some(ApciEnum::AIndividualAddressWrite),
            0x100 => Some(ApciEnum::AIndividualAddressRead),
            0x140 => Some(ApciEnum::AIndividualAddressResponse),
            0x180 => Some(ApciEnum::AAnalogToDigitalConverterRead),
            0x1C0 => Some(ApciEnum::AAnalogToDigitalConverterResponse),
            0x1C8 => Some(ApciEnum::ASystemNetworkParameterRead),
            0x1C9 => Some(ApciEnum::ASystemNetworkParameterResponse),
            0x1CA => Some(ApciEnum::ASystemNetworkParameterWrite),
            0x1CB => Some(ApciEnum::PlannedForFutureSystemBroadcastService),
            0x1CC => Some(ApciEnum::APropertyExtValueRead),
            0x1CD => Some(ApciEnum::APropertyExtValueResponse),
            0x1CE => Some(ApciEnum::APropertyExtValueWriteCon),
            0x1CF => Some(ApciEnum::APropertyExtValueWriteConRes),
            0x1D0 => Some(ApciEnum::APropertyExtValueWriteUnCon),
            0x1D1 => Some(ApciEnum::APropertyExtValueInfoReport),
            0x1D2 => Some(ApciEnum::APropertyExtDescriptionRead),
            0x1D3 => Some(ApciEnum::APropertyExtDescriptionResponse),
            0x1D4 => Some(ApciEnum::AFunctionPropertyExtCommand),
            0x1D5 => Some(ApciEnum::AFunctionPropertyExtStateRead),
            0x1D6 => Some(ApciEnum::AFunctionPropertyExtStateResponse),
            0x1FB => Some(ApciEnum::AMemoryExtendedWrite),
            0x1FC => Some(ApciEnum::AMemoryExtendedWriteResponse),
            0x1FD => Some(ApciEnum::AMemoryExtendedRead),
            0x1FE => Some(ApciEnum::AMemoryExtendedReadResponse),
            0x200 => Some(ApciEnum::AMemoryRead),
            0x240 => Some(ApciEnum::AMemoryResponse),
            0x280 => Some(ApciEnum::AMemoryWrite),
            0x2C0 => Some(ApciEnum::AUserMemoryRead),
            0x2C1 => Some(ApciEnum::AUserMemoryResponse),
            0x2C2 => Some(ApciEnum::AUserMemoryWrite),
            0x2C4 => Some(ApciEnum::AUserMemoryBitWrite),
            0x2C5 => Some(ApciEnum::AUserManufacturerInfoRead),
            0x2C6 => Some(ApciEnum::AUserManufacturerInfoResponse),
            0x2C7 => Some(ApciEnum::AFunctionPropertyCommand),
            0x2C8 => Some(ApciEnum::AFunctionPropertyStateRead),
            0x2C9 => Some(ApciEnum::AFunctionPropertyStateResponse),
            0x2CA => Some(ApciEnum::ReservedUserMsg),
            0x2F7 => Some(ApciEnum::ReservedUserMsg2),
            0x2F8 => Some(ApciEnum::ReservedUserMsg3),
            0x2FE => Some(ApciEnum::ReservedUserMsg4),
            0x300 => Some(ApciEnum::ADeviceDescriptorRead),
            0x340 => Some(ApciEnum::ADeviceDescriptorResponse),
            0x380 => Some(ApciEnum::ARestart),
            0x3C0 => Some(ApciEnum::AFilterTableOpen),
            0x3C1 => Some(ApciEnum::AFilterTableRead),
            0x3C2 => Some(ApciEnum::AFilterTableResponse),
            0x3C3 => Some(ApciEnum::AFilterTableWrite),
            0x3C8 => Some(ApciEnum::ARouterMemoryRead),
            0x3C9 => Some(ApciEnum::ARouterMemoryResponse),
            0x3CA => Some(ApciEnum::ARouterMemoryWrite),
            0x3CD => Some(ApciEnum::ARouterStatusRead),
            0x3CE => Some(ApciEnum::ARouterStatusResponse),
            0x3CF => Some(ApciEnum::ARouterStatusWrite),
            0x3D0 => Some(ApciEnum::AMemoryBitWrite),
            0x3D1 => Some(ApciEnum::AAuthorizeRequest),
            0x3D2 => Some(ApciEnum::AAuthorizeResponse),
            0x3D3 => Some(ApciEnum::AKeyWrite),
            0x3D4 => Some(ApciEnum::AKeyResponse),
            0x3D5 => Some(ApciEnum::APropertyValueRead),
            0x3D6 => Some(ApciEnum::APropertyValueResponse),
            0x3D7 => Some(ApciEnum::APropertyValueWrite),
            0x3D8 => Some(ApciEnum::APropertyDescriptionRead),
            0x3D9 => Some(ApciEnum::APropertyDescriptionResponse),
            0x3DA => Some(ApciEnum::ANetworkParameterRead),
            0x3DB => Some(ApciEnum::ANetworkParameterResponse),
            0x3DC => Some(ApciEnum::AIndividualAddressSerialNumberRead),
            0x3DD => Some(ApciEnum::AIndividualAddressSerialNumberResponse),
            0x3DE => Some(ApciEnum::AIndividualAddressSerialNumberWrite),
            0x3DF => Some(ApciEnum::Reserved2),
            0x3E0 => Some(ApciEnum::ADomainAddressWrite),
            0x3E1 => Some(ApciEnum::ADomainAddressRead),
            0x3E2 => Some(ApciEnum::ADomainAddressResponse),
            0x3E3 => Some(ApciEnum::ADomainAddressSelectiveRead),
            0x3E4 => Some(ApciEnum::ANetworkParameterWrite),
            0x3E5 => Some(ApciEnum::ALinkRead),
            0x3E6 => Some(ApciEnum::ALinkResponse),
            0x3E7 => Some(ApciEnum::ALinkWrite),
            0x3E8 => Some(ApciEnum::AGroupPropValueRead),
            0x3E9 => Some(ApciEnum::AGroupPropValueResponse),
            0x3EA => Some(ApciEnum::AGroupPropValueWrite),
            0x3EB => Some(ApciEnum::AGroupPropValueInfoReport),
            0x3EC => Some(ApciEnum::ADomainAddressSerialNumberRead),
            0x3ED => Some(ApciEnum::ADomainAddressSerialNumberResponse),
            0x3EE => Some(ApciEnum::ADomainAddressSerialNumberWrite),
            0x3F0 => Some(ApciEnum::AFileStreamInfoReport),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            ApciEnum::AGroupValueRead => "A_GroupValue_Read_Protocol_Data_Unit",
            ApciEnum::AGroupValueResponse => "A_GroupValue_Response_Protocol_Data_Unit",
            ApciEnum::AGroupValueWrite => "A_GroupValue_Write_Protocol_Data_Unit",
            ApciEnum::AIndividualAddressWrite => "A_IndividualAddress_Write_Protocol_Data_Unit",
            ApciEnum::AIndividualAddressRead => "A_IndividualAddress_Read_Protocol_Data_Unit",
            ApciEnum::AIndividualAddressResponse => "A_IndividualAddress_Response_Protocol_Data_Unit",
            ApciEnum::AAnalogToDigitalConverterRead => "A_Analog_to_Digital_Converter_Read_Protocol_Data_Unit",
            ApciEnum::AAnalogToDigitalConverterResponse => "A_Analog_to_Digital_Converter_Response_Protocol_Data_Unit",
            ApciEnum::ASystemNetworkParameterRead => "A_SystemNetworkParameter_Read_Protocol_Data_Unit",
            ApciEnum::ASystemNetworkParameterResponse => "A_SystemNetworkParameter_Response_Protocol_Data_Unit",
            ApciEnum::ASystemNetworkParameterWrite => "A_SystemNetworkParameter_Write_Protocol_Data_Unit",
            ApciEnum::PlannedForFutureSystemBroadcastService => "planned_for_future_system_broadcast_service",
            ApciEnum::APropertyExtValueRead => "A_PropertyExtValue_Read_Protocol_Data_Unit",
            ApciEnum::APropertyExtValueResponse => "A_PropertyExtValue_Response_Protocol_Data_Unit",
            ApciEnum::APropertyExtValueWriteCon => "A_PropertyExtValue_WriteCon_Protocol_Data_Unit",
            ApciEnum::APropertyExtValueWriteConRes => "A_PropertyExtValue_WriteConRes_Protocol_Data_Unit",
            ApciEnum::APropertyExtValueWriteUnCon => "A_PropertyExtValue_WriteUnCon_Protocol_Data_Unit",
            ApciEnum::APropertyExtValueInfoReport => "A_PropertyExtValue_InfoReport_Protocol_Data_Unit",
            ApciEnum::APropertyExtDescriptionRead => "A_PropertyExtDescription_Read_Protocol_Data_Unit",
            ApciEnum::APropertyExtDescriptionResponse => "A_PropertyExtDescription_Response_Protocol_Data_Unit",
            ApciEnum::AFunctionPropertyExtCommand => "A_FunctionPropertyExtCommand_Protocol_Data_Unit",
            ApciEnum::AFunctionPropertyExtStateRead => "A_FunctionPropertyExtState_Read_Protocol_Data_Unit",
            ApciEnum::AFunctionPropertyExtStateResponse => "A_FunctionPropertyExtState_Response_Protocol_Data_Unit",
            ApciEnum::AMemoryExtendedWrite => "A_MemoryExtended_Write_Protocol_Data_Unit",
            ApciEnum::AMemoryExtendedWriteResponse => "A_MemoryExtended_WriteResponse_Protocol_Data_Unit",
            ApciEnum::AMemoryExtendedRead => "A_MemoryExtended_Read_Protocol_Data_Unit",
            ApciEnum::AMemoryExtendedReadResponse => "A_MemoryExtended_ReadResponse_Protocol_Data_Unit",
            ApciEnum::AMemoryRead => "A_Memory_Read_Protocol_Data_Unit",
            ApciEnum::AMemoryResponse => "A_Memory_Response_Protocol_Data_Unit",
            ApciEnum::AMemoryWrite => "A_Memory_Write_Protocol_Data_Unit",
            ApciEnum::AUserMemoryRead => "A_UserMemory_Read_Protocol_Data_Unit",
            ApciEnum::AUserMemoryResponse => "A_UserMemory_Response_Protocol_Data_Unit",
            ApciEnum::AUserMemoryWrite => "A_UserMemory_Write_Protocol_Data_Unit",
            ApciEnum::AUserMemoryBitWrite => "A_UserMemoryBit_Write_Protocol_Data_Unit",
            ApciEnum::AUserManufacturerInfoRead => "A_UserManufacturerInfo_Read_Protocol_Data_Unit",
            ApciEnum::AUserManufacturerInfoResponse => "A_UserManufacturerInfo_Response_Protocol_Data_Unit",
            ApciEnum::AFunctionPropertyCommand => "A_FunctionPropertyCommand_Protocol_Data_Unit",
            ApciEnum::AFunctionPropertyStateRead => "A_FunctionPropertyState_Read_Protocol_Data_Unit",
            ApciEnum::AFunctionPropertyStateResponse => "A_FunctionPropertyState_Response_Protocol_Data_Unit",
            ApciEnum::ReservedUserMsg => "Reserved_USERMSG",
            ApciEnum::ReservedUserMsg2 => "Reserved_USERMSG_2",
            ApciEnum::ReservedUserMsg3 => "Reserved_USERMSG_3",
            ApciEnum::ReservedUserMsg4 => "Reserved_USERMSG_4",
            ApciEnum::ADeviceDescriptorRead => "A_DeviceDescriptor_Read_Protocol_Data_Unit",
            ApciEnum::ADeviceDescriptorResponse => "A_DeviceDescriptor_Response_Protocol_Data_Unit",
            ApciEnum::ARestart => "A_Restart_Protocol_Data_Unit",
            ApciEnum::AFilterTableOpen => "A_FilterTable_Open_Protocol_Data_Unit",
            ApciEnum::AFilterTableRead => "A_FilterTable_Read_Protocol_Data_Unit",
            ApciEnum::AFilterTableResponse => "A_FilterTable_Response_Protocol_Data_Unit",
            ApciEnum::AFilterTableWrite => "A_FilterTable_Write_Protocol_Data_Unit",
            ApciEnum::ARouterMemoryRead => "A_RouterMemory_Read_Protocol_Data_Unit",
            ApciEnum::ARouterMemoryResponse => "A_RouterMemory_Response_Protocol_Data_Unit",
            ApciEnum::ARouterMemoryWrite => "A_RouterMemory_Write_Protocol_Data_Unit",
            ApciEnum::ARouterStatusRead => "A_RouterStatus_Read_Protocol_Data_Unit",
            ApciEnum::ARouterStatusResponse => "A_RouterStatus_Response_Protocol_Data_Unit",
            ApciEnum::ARouterStatusWrite => "A_RouterStatus_Write_Protocol_Data_Unit",
            ApciEnum::AMemoryBitWrite => "A_MemoryBit_Write_Protocol_Data_Unit",
            ApciEnum::AAuthorizeRequest => "A_Authorize_Request_Protocol_Data_Unit",
            ApciEnum::AAuthorizeResponse => "A_Authorize_Response_Protocol_Data_Unit",
            ApciEnum::AKeyWrite => "A_Key_Write_Protocol_Data_Unit",
            ApciEnum::AKeyResponse => "A_Key_Response_Protocol_Data_Unit",
            ApciEnum::APropertyValueRead => "A_PropertyValue_Read_Protocol_Data_Unit",
            ApciEnum::APropertyValueResponse => "A_PropertyValue_Response_Protocol_Data_Unit",
            ApciEnum::APropertyValueWrite => "A_PropertyValue_Write_Protocol_Data_Unit",
            ApciEnum::APropertyDescriptionRead => "A_PropertyDescription_Read_Protocol_Data_Unit",
            ApciEnum::APropertyDescriptionResponse => "A_PropertyDescription_Response_Protocol_Data_Unit",
            ApciEnum::ANetworkParameterRead => "A_NetworkParameter_Read_Protocol_Data_Unit",
            ApciEnum::ANetworkParameterResponse => "A_NetworkParameter_Response_Protocol_Data_Unit",
            ApciEnum::AIndividualAddressSerialNumberRead => "A_IndividualAddressSerialNumber_Read_Protocol_Data_Unit",
            ApciEnum::AIndividualAddressSerialNumberResponse => "A_IndividualAddressSerialNumber_Response_Protocol_Data_Unit",
            ApciEnum::AIndividualAddressSerialNumberWrite => "A_IndividualAddressSerialNumber_Write_Protocol_Data_Unit",
            ApciEnum::Reserved2 => "Reserved_2",
            ApciEnum::ADomainAddressWrite => "A_DomainAddress_Write_Protocol_Data_Unit",
            ApciEnum::ADomainAddressRead => "A_DomainAddress_Read_Protocol_Data_Unit",
            ApciEnum::ADomainAddressResponse => "A_DomainAddress_Response_Protocol_Data_Unit",
            ApciEnum::ADomainAddressSelectiveRead => "A_DomainAddressSelective_Read_Protocol_Data_Unit",
            ApciEnum::ANetworkParameterWrite => "A_NetworkParameter_Write_Protocol_Data_Unit",
            ApciEnum::ALinkRead => "A_Link_Read_Protocol_Data_Unit",
            ApciEnum::ALinkResponse => "A_Link_Response_Protocol_Data_Unit",
            ApciEnum::ALinkWrite => "A_Link_Write_Protocol_Data_Unit",
            ApciEnum::AGroupPropValueRead => "A_GroupPropValue_Read_Protocol_Data_Unit",
            ApciEnum::AGroupPropValueResponse => "A_GroupPropValue_Response_Protocol_Data_Unit",
            ApciEnum::AGroupPropValueWrite => "A_GroupPropValue_Write_Protocol_Data_Unit",
            ApciEnum::AGroupPropValueInfoReport => "A_GroupPropValue_InfoReport_Protocol_Data_Unit",
            ApciEnum::ADomainAddressSerialNumberRead => "A_DomainAddressSerialNumber_Read_Protocol_Data_Unit",
            ApciEnum::ADomainAddressSerialNumberResponse => "A_DomainAddressSerialNumber_Response_Protocol_Data_Unit",
            ApciEnum::ADomainAddressSerialNumberWrite => "A_DomainAddressSerialNumber_Write_Protocol_Data_Unit",
            ApciEnum::AFileStreamInfoReport => "A_FileStream_InfoReport_Protocol_Data_Unit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Apci {
    value: u16,
}

impl Apci {
    pub fn new(apci: u16) -> Self {
        Self { value: apci & 0x03FF }
    }

    pub fn get_value(&self) -> u16 {
        self.value
    }

    pub fn set_value(&mut self, val: u16) {
        self.value = val & 0x03FF;
    }

    pub fn get_command(&self) -> String {
        match ApciEnum::from_u16(self.value) {
            Some(e) => e.to_str().to_string(),
            None => format!("UNKNOWN_APCI_COMMAND_{}", self.value),
        }
    }

    pub fn set_command(&mut self, cmd: ApciEnum) {
        self.value = cmd as u16;
    }

    pub fn to_hex(&self) -> String {
        format!("0x{:02X}", self.value)
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        vec![(self.value >> 8) as u8, (self.value & 0xFF) as u8]
    }

    pub fn pack_number(&self) -> (u8, u8) {
        let high = ((self.value >> 8) & 0x03) as u8;
        let low = (self.value & 0xFF) as u8;
        (high, low)
    }

    pub fn unpack_number(octet1: u8, octet2: u8) -> u16 {
        let high = (octet1 & 0x03) as u16;
        let low = octet2 as u16;
        (high << 8) | low
    }

    pub fn describe(&self) -> ApciDescription {
        ApciDescription {
            obj: "APCI",
            command: self.get_command(),
            value: format!(
                "{} (Buffer: {:02X}{:02X})",
                self.to_hex(),
                (self.value >> 8) as u8,
                (self.value & 0xFF) as u8
            ),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ApciDescription {
    pub obj: &'static str,
    pub command: String,
    pub value: String,
}
