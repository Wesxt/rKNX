use crate::core::control_field::{ControlField, ControlFieldDescription};
use crate::core::control_field_extended::{ExtendedControlField, ExtendedControlFieldDescription};
use crate::core::knx_add_info_types::AddInfo;
use crate::core::layers::data::tpdu::{Tpdu, TpduDescription};
use crate::errors::KnxError;
use crate::utils::knx_helper::KnxHelper;
use byteorder::{BigEndian, ByteOrder};

// ==========================================
// Additional Information Field Helper
// ==========================================
#[derive(Debug, Clone)]
pub struct AdditionalInformationField {
    pub items: Vec<AddInfo>,
}

impl AdditionalInformationField {
    pub fn new(items: Vec<AddInfo>) -> Self {
        Self { items }
    }

    pub fn default() -> Self {
        Self { items: Vec::new() }
    }

    pub fn describe(&self) -> AdditionalInfoDescription {
        AdditionalInfoDescription {
            obj: "AdditionalInformationField",
            items: self.items.iter().map(|item| {
                match item {
                    AddInfo::PLMediumInfo(x) => format!("{:?}", x.describe()),
                    AddInfo::RFMediumInformation(x) => format!("{:?}", x.describe()),
                    AddInfo::BusmonitorStatusInfo(x) => format!("{:?}", x.describe()),
                    AddInfo::TimestampRelative(x) => format!("{:?}", x.describe()),
                    AddInfo::TimeDelayUntilSending(x) => format!("{:?}", x.describe()),
                    AddInfo::ExtendedRelativeTimestamp(x) => format!("{:?}", x.describe()),
                    AddInfo::BiBatInformation(x) => format!("{:?}", x.describe()),
                    AddInfo::RFMultiInformation(x) => format!("{:?}", x.describe()),
                    AddInfo::PreambleAndPostamble(x) => format!("{:?}", x.describe()),
                    AddInfo::RFFastACKInformation(x) => format!("{:?}", x.describe()),
                    AddInfo::ManufacturerSpecificData(x) => format!("{:?}", x.describe()),
                }
            }).collect(),
        }
    }

    pub fn length(&self) -> usize {
        self.items.iter().map(|item| item.total_length()).sum()
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        for item in &self.items {
            buffer.extend_from_slice(&item.get_buffer());
        }
        buffer
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        let mut items = Vec::new();
        let mut offset = 0;
        while offset < buffer.len() {
            if offset + 2 > buffer.len() {
                return Err(KnxError::InvalidParametersForDpt);
            }
            let len = buffer[offset + 1] as usize;
            let total_block_size = 2 + len;
            if offset + total_block_size > buffer.len() {
                return Err(KnxError::InvalidParametersForDpt);
            }
            let block = &buffer[offset..offset + total_block_size];
            if let Ok(item) = AddInfo::from_buffer(block) {
                items.push(item);
            }
            offset += total_block_size;
        }
        Ok(Self::new(items))
    }
}

// ==========================================
// CEMI Specific Structures
// ==========================================
#[derive(Debug, Clone)]
pub struct LData {
    pub additional_info: Vec<AddInfo>,
    pub control_field1: ControlField,
    pub control_field2: ExtendedControlField,
    pub source_address: String,
    pub destination_address: String,
    pub tpdu: Tpdu,
}

#[derive(Debug, Clone)]
pub struct LPollData {
    pub additional_info: Vec<AddInfo>,
    pub control_field1: ControlField,
    pub control_field2: ExtendedControlField,
    pub source_address: String,
    pub destination_address: String,
    pub num_of_slots: u8,
    pub poll_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LRaw {
    pub additional_info: Vec<AddInfo>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LBusmon {
    pub additional_info: Vec<AddInfo>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TDataConnected {
    pub additional_info: Vec<AddInfo>,
    pub tpdu: Tpdu,
}

#[derive(Debug, Clone)]
pub struct MProp {
    pub interface_object_type: u16,
    pub object_instance: u8,
    pub property_id: u8,
    pub number_of_elements: u8,
    pub start_index: u16,
}

#[derive(Debug, Clone)]
pub struct MPropWithPayload {
    pub interface_object_type: u16,
    pub object_instance: u8,
    pub property_id: u8,
    pub number_of_elements: u8,
    pub start_index: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MPropWriteConfirm {
    pub interface_object_type: u16,
    pub object_instance: u8,
    pub property_id: u8,
    pub number_of_elements: u8,
    pub start_index: u16,
    pub error_info: u8,
}

#[derive(Debug, Clone)]
pub struct MFuncProp {
    pub interface_object_type: u16,
    pub object_instance: u8,
    pub property_id: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MFuncPropWithReturn {
    pub interface_object_type: u16,
    pub object_instance: u8,
    pub property_id: u8,
    pub return_code: u8,
    pub data: Vec<u8>,
}

// ==========================================
// Cemi Enum Wrapper
// ==========================================
#[derive(Debug, Clone)]
pub enum Cemi {
    LDataReq(LData),
    LDataCon(LData),
    LDataInd(LData),
    LPollDataReq(LPollData),
    LPollDataCon(LPollData),
    LRawReq(LRaw),
    LRawCon(LRaw),
    LRawInd(LRaw),
    LBusmonInd(LBusmon),
    TDataConnectedReq(TDataConnected),
    TDataConnectedInd(TDataConnected),
    MPropReadReq(MProp),
    MPropReadCon(MPropWithPayload),
    MPropWriteReq(MPropWithPayload),
    MPropWriteCon(MPropWriteConfirm),
    MPropInfoInd(MPropWithPayload),
    MFuncPropCommandReq(MFuncProp),
    MFuncPropCommandCon(MFuncPropWithReturn),
    MFuncPropStateReadReq(MFuncProp),
    MResetReq,
    MResetInd,
}

impl Cemi {
    pub fn get_message_code(&self) -> u8 {
        let name = match self {
            Cemi::LDataReq(_) => "L_Data.req",
            Cemi::LDataCon(_) => "L_Data.con",
            Cemi::LDataInd(_) => "L_Data.ind",
            Cemi::LPollDataReq(_) => "L_Poll_Data.req",
            Cemi::LPollDataCon(_) => "L_Poll_Data.con",
            Cemi::LRawReq(_) => "L_Raw.req",
            Cemi::LRawCon(_) => "L_Raw.con",
            Cemi::LRawInd(_) => "L_Raw.ind",
            Cemi::LBusmonInd(_) => "L_Busmon.ind",
            Cemi::TDataConnectedReq(_) => "T_Data_Connected.req",
            Cemi::TDataConnectedInd(_) => "T_Data_Connected.ind",
            Cemi::MPropReadReq(_) => "M_PropRead.req",
            Cemi::MPropReadCon(_) => "M_PropRead.con",
            Cemi::MPropWriteReq(_) => "M_PropWrite.req",
            Cemi::MPropWriteCon(_) => "M_PropWrite.con",
            Cemi::MPropInfoInd(_) => "M_PropInfo.ind",
            Cemi::MFuncPropCommandReq(_) => "M_FuncPropCommand.req",
            Cemi::MFuncPropCommandCon(_) => "M_FuncPropCommand.con",
            Cemi::MFuncPropStateReadReq(_) => "M_FuncPropStateRead.req",
            Cemi::MResetReq => "M_Reset.req",
            Cemi::MResetInd => "M_Reset.ind",
        };
        crate::core::message_code_field::get_cemi_message_code(name).unwrap_or(0)
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        match self {
            Cemi::LDataReq(ld) | Cemi::LDataCon(ld) | Cemi::LDataInd(ld) => {
                let add_info_field = AdditionalInformationField::new(ld.additional_info.clone());
                let base_offset = 2 + add_info_field.length();
                let tpdu_buffer = ld.tpdu.to_buffer();

                let mut buffer = vec![0u8; base_offset + 7 + tpdu_buffer.len()];
                buffer[0] = self.get_message_code();
                buffer[1] = add_info_field.length() as u8;

                if add_info_field.length() > 0 {
                    buffer[2..base_offset].copy_from_slice(&add_info_field.to_buffer());
                }

                buffer[base_offset] = ld.control_field1.get_buffer()[0];
                buffer[base_offset + 1] = ld.control_field2.get_buffer()[0];

                let src_bytes =
                    KnxHelper::get_address_from_string(&ld.source_address).unwrap_or_default();
                buffer[base_offset + 2..base_offset + 4].copy_from_slice(&src_bytes);

                let dst_bytes =
                    KnxHelper::get_address_from_string(&ld.destination_address).unwrap_or_default();
                buffer[base_offset + 4..base_offset + 6].copy_from_slice(&dst_bytes);

                buffer[base_offset + 6] = ld.tpdu.apdu.get_length() as u8;
                buffer[base_offset + 7..base_offset + 7 + tpdu_buffer.len()]
                    .copy_from_slice(&tpdu_buffer);
                buffer
            }
            Cemi::LPollDataReq(pd) | Cemi::LPollDataCon(pd) => {
                let add_info_field = AdditionalInformationField::new(pd.additional_info.clone());
                let base_offset = 2 + add_info_field.length();

                let extra = if matches!(self, Cemi::LPollDataCon(_)) {
                    8
                } else {
                    7
                };
                let mut buffer = vec![0u8; base_offset + extra];
                buffer[0] = self.get_message_code();
                buffer[1] = add_info_field.length() as u8;

                if add_info_field.length() > 0 {
                    buffer[2..base_offset].copy_from_slice(&add_info_field.to_buffer());
                }

                buffer[base_offset] = pd.control_field1.get_buffer()[0];
                buffer[base_offset + 1] = pd.control_field2.get_buffer()[0];

                let src_bytes =
                    KnxHelper::get_address_from_string(&pd.source_address).unwrap_or_default();
                buffer[base_offset + 2..base_offset + 4].copy_from_slice(&src_bytes);

                let dst_bytes =
                    KnxHelper::get_address_from_string(&pd.destination_address).unwrap_or_default();
                buffer[base_offset + 4..base_offset + 6].copy_from_slice(&dst_bytes);

                buffer[base_offset + 6] = pd.num_of_slots;
                if matches!(self, Cemi::LPollDataCon(_)) && !pd.poll_data.is_empty() {
                    buffer[base_offset + 7] = pd.poll_data[0];
                }
                buffer
            }
            Cemi::LRawReq(lr) | Cemi::LRawCon(lr) | Cemi::LRawInd(lr) => {
                let add_info_field = AdditionalInformationField::new(lr.additional_info.clone());
                let base_offset = 2 + add_info_field.length();

                let mut buffer = vec![0u8; base_offset + lr.data.len()];
                buffer[0] = self.get_message_code();
                buffer[1] = add_info_field.length() as u8;

                if add_info_field.length() > 0 {
                    buffer[2..base_offset].copy_from_slice(&add_info_field.to_buffer());
                }

                buffer[base_offset..].copy_from_slice(&lr.data);
                buffer
            }
            Cemi::LBusmonInd(lb) => {
                let add_info_field = AdditionalInformationField::new(lb.additional_info.clone());
                let base_offset = 2 + add_info_field.length();

                let mut buffer = vec![0u8; base_offset + lb.data.len()];
                buffer[0] = self.get_message_code();
                buffer[1] = add_info_field.length() as u8;

                if add_info_field.length() > 0 {
                    buffer[2..base_offset].copy_from_slice(&add_info_field.to_buffer());
                }

                buffer[base_offset..].copy_from_slice(&lb.data);
                buffer
            }
            Cemi::TDataConnectedReq(tc) | Cemi::TDataConnectedInd(tc) => {
                let add_info_field = AdditionalInformationField::new(tc.additional_info.clone());
                let base_offset = 2 + add_info_field.length();
                let tpdu_buffer = tc.tpdu.to_buffer();

                let mut buffer = vec![0u8; base_offset + 6 + 1 + tpdu_buffer.len()];
                buffer[0] = self.get_message_code();
                buffer[1] = add_info_field.length() as u8;

                if add_info_field.length() > 0 {
                    buffer[2..base_offset].copy_from_slice(&add_info_field.to_buffer());
                }

                buffer[base_offset + 6] = tc.tpdu.apdu.get_length() as u8;
                buffer[base_offset + 7..base_offset + 7 + tpdu_buffer.len()]
                    .copy_from_slice(&tpdu_buffer);
                buffer
            }
            Cemi::MPropReadReq(mp) => {
                let mut buffer = vec![0u8; 7];
                buffer[0] = self.get_message_code();
                BigEndian::write_u16(&mut buffer[1..3], mp.interface_object_type);
                buffer[3] = mp.object_instance;
                buffer[4] = mp.property_id;

                let val5 =
                    (mp.start_index & 0x0FFF) | ((mp.number_of_elements as u16 & 0x0F) << 12);
                BigEndian::write_u16(&mut buffer[5..7], val5);
                buffer
            }
            Cemi::MPropReadCon(mp) | Cemi::MPropWriteReq(mp) | Cemi::MPropInfoInd(mp) => {
                let mut buffer = vec![0u8; 7 + mp.data.len()];
                buffer[0] = self.get_message_code();
                BigEndian::write_u16(&mut buffer[1..3], mp.interface_object_type);
                buffer[3] = mp.object_instance;
                buffer[4] = mp.property_id;

                let val5 =
                    (mp.start_index & 0x0FFF) | ((mp.number_of_elements as u16 & 0x0F) << 12);
                BigEndian::write_u16(&mut buffer[5..7], val5);
                buffer[7..].copy_from_slice(&mp.data);
                buffer
            }
            Cemi::MPropWriteCon(mp) => {
                let size = if mp.number_of_elements == 0 { 8 } else { 7 };
                let mut buffer = vec![0u8; size];
                buffer[0] = self.get_message_code();
                BigEndian::write_u16(&mut buffer[1..3], mp.interface_object_type);
                buffer[3] = mp.object_instance;
                buffer[4] = mp.property_id;

                let val5 =
                    (mp.start_index & 0x0FFF) | ((mp.number_of_elements as u16 & 0x0F) << 12);
                BigEndian::write_u16(&mut buffer[5..7], val5);
                if mp.number_of_elements == 0 {
                    buffer[7] = mp.error_info;
                }
                buffer
            }
            Cemi::MFuncPropCommandReq(mf) | Cemi::MFuncPropStateReadReq(mf) => {
                let mut buffer = vec![0u8; 5 + mf.data.len()];
                buffer[0] = self.get_message_code();
                BigEndian::write_u16(&mut buffer[1..3], mf.interface_object_type);
                buffer[3] = mf.object_instance;
                buffer[4] = mf.property_id;
                buffer[5..].copy_from_slice(&mf.data);
                buffer
            }
            Cemi::MFuncPropCommandCon(mf) => {
                let mut buffer = vec![0u8; 6 + mf.data.len()];
                buffer[0] = self.get_message_code();
                BigEndian::write_u16(&mut buffer[1..3], mf.interface_object_type);
                buffer[3] = mf.object_instance;
                buffer[4] = mf.property_id;
                buffer[5] = mf.return_code;
                buffer[6..].copy_from_slice(&mf.data);
                buffer
            }
            Cemi::MResetReq | Cemi::MResetInd => {
                vec![self.get_message_code()]
            }
        }
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.is_empty() {
            return Err(KnxError::InvalidParametersForDpt);
        }
        let message_code = buffer[0];
        let service_name = crate::core::message_code_field::get_service_name_by_cemi_value(message_code)
            .ok_or(KnxError::InvalidParametersForDpt)?;

        match service_name {
            "L_Data.req" | "L_Data.con" | "L_Data.ind" => {
                if buffer.len() < 2 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let add_info_len = buffer[1] as usize;
                let base_offset = 2 + add_info_len;
                if buffer.len() < base_offset + 7 {
                    return Err(KnxError::InvalidParametersForDpt);
                }

                let additional_info = if add_info_len > 0 {
                    AdditionalInformationField::from_buffer(&buffer[2..base_offset])?.items
                } else {
                    Vec::new()
                };

                let control_field1 = ControlField::new(buffer[base_offset]);
                let control_field2 = ExtendedControlField::new(buffer[base_offset + 1]);

                let source_address = KnxHelper::get_address_to_string(
                    &buffer[base_offset + 2..base_offset + 4],
                    ".",
                    false,
                )?;
                let is_group = control_field2.get_address_type()
                    == crate::core::control_field_extended::AddressType::Group;
                let destination_address = KnxHelper::get_address_to_string(
                    &buffer[base_offset + 4..base_offset + 6],
                    if is_group { "/" } else { "." },
                    is_group,
                )?;

                let length = buffer[base_offset + 6] as usize;
                // APDU length is specified, TPDU has 1 more octet for TPCI
                if buffer.len() < base_offset + 7 + length + 1 {
                    return Err(KnxError::InvalidParametersForDpt);
                }

                let tpdu_buffer = &buffer[base_offset + 7..base_offset + 7 + length + 1];
                let tpdu = Tpdu::from_buffer(tpdu_buffer)?;

                let ld = LData {
                    additional_info,
                    control_field1,
                    control_field2,
                    source_address,
                    destination_address,
                    tpdu,
                };

                if service_name == "L_Data.req" {
                    Ok(Cemi::LDataReq(ld))
                } else if service_name == "L_Data.con" {
                    Ok(Cemi::LDataCon(ld))
                } else {
                    Ok(Cemi::LDataInd(ld))
                }
            }
            "L_Poll_Data.req" | "L_Poll_Data.con" => {
                if buffer.len() < 2 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let add_info_len = buffer[1] as usize;
                let base_offset = 2 + add_info_len;
                let extra = if service_name == "L_Poll_Data.con" { 8 } else { 7 };
                if buffer.len() < base_offset + extra {
                    return Err(KnxError::InvalidParametersForDpt);
                }

                let additional_info = if add_info_len > 0 {
                    AdditionalInformationField::from_buffer(&buffer[2..base_offset])?.items
                } else {
                    Vec::new()
                };

                let control_field1 = ControlField::new(buffer[base_offset]);
                let control_field2 = ExtendedControlField::new(buffer[base_offset + 1]);

                let source_address = KnxHelper::get_address_to_string(
                    &buffer[base_offset + 2..base_offset + 4],
                    ".",
                    false,
                )?;
                let is_group = control_field2.get_address_type()
                    == crate::core::control_field_extended::AddressType::Group;
                let destination_address = KnxHelper::get_address_to_string(
                    &buffer[base_offset + 4..base_offset + 6],
                    if is_group { "/" } else { "." },
                    is_group,
                )?;

                let num_of_slots = buffer[base_offset + 6];
                let poll_data = if service_name == "L_Poll_Data.con" {
                    vec![buffer[base_offset + 7]]
                } else {
                    Vec::new()
                };

                let pd = LPollData {
                    additional_info,
                    control_field1,
                    control_field2,
                    source_address,
                    destination_address,
                    num_of_slots,
                    poll_data,
                };

                if service_name == "L_Poll_Data.req" {
                    Ok(Cemi::LPollDataReq(pd))
                } else {
                    Ok(Cemi::LPollDataCon(pd))
                }
            }
            "L_Raw.req" | "L_Raw.con" | "L_Raw.ind" => {
                if buffer.len() < 2 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let add_info_len = buffer[1] as usize;
                let base_offset = 2 + add_info_len;
                if buffer.len() < base_offset {
                    return Err(KnxError::InvalidParametersForDpt);
                }

                let additional_info = if add_info_len > 0 {
                    AdditionalInformationField::from_buffer(&buffer[2..base_offset])?.items
                } else {
                    Vec::new()
                };

                let data = buffer[base_offset..].to_vec();
                let lr = LRaw {
                    additional_info,
                    data,
                };

                if service_name == "L_Raw.req" {
                    Ok(Cemi::LRawReq(lr))
                } else if service_name == "L_Raw.con" {
                    Ok(Cemi::LRawCon(lr))
                } else {
                    Ok(Cemi::LRawInd(lr))
                }
            }
            "L_Busmon.ind" => {
                if buffer.len() < 2 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let add_info_len = buffer[1] as usize;
                let base_offset = 2 + add_info_len;
                if buffer.len() < base_offset {
                    return Err(KnxError::InvalidParametersForDpt);
                }

                let additional_info = if add_info_len > 0 {
                    AdditionalInformationField::from_buffer(&buffer[2..base_offset])?.items
                } else {
                    Vec::new()
                };

                let data = buffer[base_offset..].to_vec();
                Ok(Cemi::LBusmonInd(LBusmon {
                    additional_info,
                    data,
                }))
            }
            "T_Data_Connected.req" | "T_Data_Connected.ind" => {
                if buffer.len() < 2 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let add_info_len = buffer[1] as usize;
                let base_offset = 2 + add_info_len;
                if buffer.len() < base_offset + 7 {
                    return Err(KnxError::InvalidParametersForDpt);
                }

                let additional_info = if add_info_len > 0 {
                    AdditionalInformationField::from_buffer(&buffer[2..base_offset])?.items
                } else {
                    Vec::new()
                };

                let tpdu = Tpdu::from_buffer(&buffer[base_offset + 7..])?;
                let tc = TDataConnected {
                    additional_info,
                    tpdu,
                };

                if service_name == "T_Data_Connected.req" {
                    Ok(Cemi::TDataConnectedReq(tc))
                } else {
                    Ok(Cemi::TDataConnectedInd(tc))
                }
            }
            "M_PropRead.req" => {
                if buffer.len() < 7 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let interface_object_type = BigEndian::read_u16(&buffer[1..3]);
                let object_instance = buffer[3];
                let property_id = buffer[4];

                let val5 = BigEndian::read_u16(&buffer[5..7]);
                let start_index = val5 & 0x0FFF;
                let number_of_elements = ((val5 >> 12) & 0x0F) as u8;

                Ok(Cemi::MPropReadReq(MProp {
                    interface_object_type,
                    object_instance,
                    property_id,
                    number_of_elements,
                    start_index,
                }))
            }
            "M_PropRead.con" | "M_PropWrite.req" | "M_PropInfo.ind" => {
                if buffer.len() < 7 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let interface_object_type = BigEndian::read_u16(&buffer[1..3]);
                let object_instance = buffer[3];
                let property_id = buffer[4];

                let val5 = BigEndian::read_u16(&buffer[5..7]);
                let start_index = val5 & 0x0FFF;
                let number_of_elements = ((val5 >> 12) & 0x0F) as u8;
                let data = buffer[7..].to_vec();

                let mp = MPropWithPayload {
                    interface_object_type,
                    object_instance,
                    property_id,
                    number_of_elements,
                    start_index,
                    data,
                };

                if service_name == "M_PropRead.con" {
                    Ok(Cemi::MPropReadCon(mp))
                } else if service_name == "M_PropWrite.req" {
                    Ok(Cemi::MPropWriteReq(mp))
                } else {
                    Ok(Cemi::MPropInfoInd(mp))
                }
            }
            "M_PropWrite.con" => {
                if buffer.len() < 7 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let interface_object_type = BigEndian::read_u16(&buffer[1..3]);
                let object_instance = buffer[3];
                let property_id = buffer[4];

                let val5 = BigEndian::read_u16(&buffer[5..7]);
                let start_index = val5 & 0x0FFF;
                let number_of_elements = ((val5 >> 12) & 0x0F) as u8;

                let mut error_info = 0;
                if number_of_elements == 0 && buffer.len() >= 8 {
                    error_info = buffer[7];
                }

                Ok(Cemi::MPropWriteCon(MPropWriteConfirm {
                    interface_object_type,
                    object_instance,
                    property_id,
                    number_of_elements,
                    start_index,
                    error_info,
                }))
            }
            "M_FuncPropCommand.req" | "M_FuncPropStateRead.req" => {
                if buffer.len() < 5 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let interface_object_type = BigEndian::read_u16(&buffer[1..3]);
                let object_instance = buffer[3];
                let property_id = buffer[4];
                let data = buffer[5..].to_vec();

                let mf = MFuncProp {
                    interface_object_type,
                    object_instance,
                    property_id,
                    data,
                };

                if service_name == "M_FuncPropCommand.req" {
                    Ok(Cemi::MFuncPropCommandReq(mf))
                } else {
                    Ok(Cemi::MFuncPropStateReadReq(mf))
                }
            }
            "M_FuncPropCommand.con" => {
                if buffer.len() < 6 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let interface_object_type = BigEndian::read_u16(&buffer[1..3]);
                let object_instance = buffer[3];
                let property_id = buffer[4];
                let return_code = buffer[5];
                let data = buffer[6..].to_vec();

                Ok(Cemi::MFuncPropCommandCon(MFuncPropWithReturn {
                    interface_object_type,
                    object_instance,
                    property_id,
                    return_code,
                    data,
                }))
            }
            "M_Reset.req" => Ok(Cemi::MResetReq),
            "M_Reset.ind" => Ok(Cemi::MResetInd),
            _ => Err(KnxError::InvalidParametersForDpt),
        }
    }

    pub fn describe(&self, node_format: bool) -> String {
        let desc = self.describe_struct();
        if node_format {
            desc.to_node_string()
        } else {
            format!("{:?}", desc)
        }
    }

    pub fn describe_struct(&self) -> CemiDescription {
        match self {
            Cemi::LDataReq(data) | Cemi::LDataCon(data) | Cemi::LDataInd(data) => {
                let obj = match self {
                    Cemi::LDataReq(_) => "L_Data_req",
                    Cemi::LDataCon(_) => "L_Data_con",
                    Cemi::LDataInd(_) => "L_Data_ind",
                    _ => unreachable!(),
                };
                let add_info_field = AdditionalInformationField::new(data.additional_info.clone());
                CemiDescription {
                    obj: obj.to_string(),
                    message_code: self.get_message_code(),
                    additional_info: Some(add_info_field.describe()),
                    control_field1: Some(data.control_field1.describe()),
                    control_field2: Some(data.control_field2.describe()),
                    source_address: Some(data.source_address.clone()),
                    destination_address: Some(data.destination_address.clone()),
                    tpdu: Some(data.tpdu.describe()),
                }
            }
            Cemi::LPollDataReq(data) | Cemi::LPollDataCon(data) => {
                let obj = match self {
                    Cemi::LPollDataReq(_) => "L_Poll_Data_req",
                    Cemi::LPollDataCon(_) => "L_Poll_Data_con",
                    _ => unreachable!(),
                };
                let add_info_field = AdditionalInformationField::new(data.additional_info.clone());
                CemiDescription {
                    obj: obj.to_string(),
                    message_code: self.get_message_code(),
                    additional_info: Some(add_info_field.describe()),
                    control_field1: Some(data.control_field1.describe()),
                    control_field2: Some(data.control_field2.describe()),
                    source_address: Some(data.source_address.clone()),
                    destination_address: Some(data.destination_address.clone()),
                    tpdu: None,
                }
            }
            Cemi::LBusmonInd(data) => {
                let add_info_field = AdditionalInformationField::new(data.additional_info.clone());
                CemiDescription {
                    obj: "L_Busmon_ind".to_string(),
                    message_code: self.get_message_code(),
                    additional_info: Some(add_info_field.describe()),
                    control_field1: None,
                    control_field2: None,
                    source_address: None,
                    destination_address: None,
                    tpdu: None,
                }
            }
            _ => {
                let obj = match self {
                    Cemi::LRawReq(_) => "L_Raw_req",
                    Cemi::LRawCon(_) => "L_Raw_con",
                    Cemi::LRawInd(_) => "L_Raw_ind",
                    Cemi::TDataConnectedReq(_) => "T_Data_Connected_req",
                    Cemi::TDataConnectedInd(_) => "T_Data_Connected_ind",
                    Cemi::MPropReadReq(_) => "M_Prop_Read_req",
                    Cemi::MPropReadCon(_) => "M_Prop_Read_con",
                    Cemi::MPropWriteReq(_) => "M_Prop_Write_req",
                    Cemi::MPropWriteCon(_) => "M_Prop_Write_con",
                    Cemi::MPropInfoInd(_) => "M_Prop_Info_ind",
                    Cemi::MFuncPropCommandReq(_) => "M_Func_Prop_Command_req",
                    Cemi::MFuncPropCommandCon(_) => "M_Func_Prop_Command_con",
                    Cemi::MFuncPropStateReadReq(_) => "M_Func_Prop_State_Read_req",
                    Cemi::MResetReq => "M_Reset_req",
                    Cemi::MResetInd => "M_Reset_ind",
                    _ => "Cemi",
                };
                CemiDescription {
                    obj: obj.to_string(),
                    message_code: self.get_message_code(),
                    additional_info: None,
                    control_field1: None,
                    control_field2: None,
                    source_address: None,
                    destination_address: None,
                    tpdu: None,
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CemiDescription {
    pub obj: String,
    pub message_code: u8,
    pub additional_info: Option<AdditionalInfoDescription>,
    pub control_field1: Option<ControlFieldDescription>,
    pub control_field2: Option<ExtendedControlFieldDescription>,
    pub source_address: Option<String>,
    pub destination_address: Option<String>,
    pub tpdu: Option<TpduDescription>,
}

impl CemiDescription {
    pub fn to_node_string(&self) -> String {
        let mut out = String::new();
        out.push_str("CEMI {\n");
        out.push_str(&format!("  obj: '{}',\n", self.obj));
        out.push_str(&format!("  messageCode: {},\n", self.message_code));
        
        if let Some(ref add_info) = self.additional_info {
            out.push_str("  additionalInfo: {\n");
            out.push_str(&format!("    obj: '{}',\n", add_info.obj));
            out.push_str("    items: [");
            if !add_info.items.is_empty() {
                out.push('\n');
                for (i, item) in add_info.items.iter().enumerate() {
                    let formatted = item.replace("\n", "\n      ");
                    out.push_str(&format!("      {}", formatted));
                    if i < add_info.items.len() - 1 {
                        out.push_str(",\n");
                    } else {
                        out.push('\n');
                    }
                }
                out.push_str("    ]\n");
            } else {
                out.push_str("]\n");
            }
            out.push_str("  },\n");
        }

        if let Some(ref cf1) = self.control_field1 {
            out.push_str("  controlField1: {\n");
            out.push_str(&format!("    obj: '{}',\n", cf1.obj));
            let clean_hex = cf1.hex.trim_start_matches("0x").to_lowercase();
            out.push_str(&format!("    buffer: <Buffer {}>,\n", clean_hex));
            out.push_str(&format!("    hex: '{}',\n", cf1.hex.to_lowercase()));
            out.push_str(&format!("    FrameType: '{}',\n", cf1.frame_type));
            out.push_str(&format!("    Repeat: {},\n", cf1.repeat));
            out.push_str(&format!("    SystemBroadcast: {},\n", cf1.system_broadcast));
            out.push_str(&format!("    Priority: '{}',\n", cf1.priority));
            out.push_str(&format!("    ACKRequest: '{}',\n", cf1.ack_request));
            out.push_str(&format!("    Confirm: '{}'\n", cf1.confirm));
            out.push_str("  },\n");
        }

        if let Some(ref cf2) = self.control_field2 {
            out.push_str("  controlField2: {\n");
            out.push_str(&format!("    obj: '{}',\n", cf2.obj));
            out.push_str(&format!("    hex: '{}',\n", cf2.hex.to_lowercase()));
            out.push_str(&format!("    addressType: '{}',\n", cf2.address_type));
            out.push_str(&format!("    hopCount: {},\n", cf2.hop_count));
            let eff_num = match cf2.extended_frame_format {
                "PointToPointOrStandardGroupAddressed" => 0,
                "MulticastZoneAddressed11" => 7,
                "MulticastZoneAddressed01" => 5,
                "MulticastZoneAddressed10" => 6,
                "MulticastZoneAddressed00" => 4,
                _ => 0,
            };
            out.push_str(&format!("    extendedFrameFormat: {},\n", eff_num));
            out.push_str(&format!("    buffer: {}\n", cf2.buffer));
            out.push_str("  },\n");
        }

        if let Some(ref src) = self.source_address {
            out.push_str(&format!("  sourceAddress: '{}',\n", src));
        }

        if let Some(ref dst) = self.destination_address {
            out.push_str(&format!("  destinationAddress: '{}',\n", dst));
        }

        if let Some(ref tpdu) = self.tpdu {
            out.push_str("  TPDU: {\n");
            out.push_str(&format!("    obj: '{}',\n", tpdu.obj));
            out.push_str(&format!("    layer: '{}',\n", tpdu.layer));
            
            // tpci
            out.push_str("    tpci: {\n");
            out.push_str(&format!("      obj: '{}',\n", tpdu.tpci.obj));
            let tpci_clean_hex = tpdu.tpci.hex.trim_start_matches("0x").to_lowercase();
            out.push_str(&format!("      buffer: <Buffer {}>,\n", tpci_clean_hex));
            out.push_str(&format!("      hex: '{}',\n", tpdu.tpci.hex.to_lowercase()));
            out.push_str(&format!("      dataOrControlFlag: '{}',\n", tpdu.tpci.data_or_control_flag));
            out.push_str(&format!("      numbered: {},\n", tpdu.tpci.numbered));
            out.push_str(&format!("      sequenceNumber: {},\n", tpdu.tpci.sequence_number));
            out.push_str(&format!("      firstTwoBitsFromAPCI: {},\n", tpdu.tpci.first_two_bits_from_apci));
            out.push_str(&format!("      TPCIType: '{}'\n", tpdu.tpci.tpci_type));
            out.push_str("    },\n");

            // apdu
            out.push_str("    APDU: {\n");
            out.push_str(&format!("      obj: '{}',\n", tpdu.apdu.obj));
            out.push_str(&format!("      layer: '{}',\n", tpdu.apdu.layer));
            
            out.push_str("      tpci: {\n");
            out.push_str(&format!("        obj: '{}',\n", tpdu.apdu.tpci.obj));
            let apdu_tpci_clean_hex = tpdu.apdu.tpci.hex.trim_start_matches("0x").to_lowercase();
            out.push_str(&format!("        buffer: <Buffer {}>,\n", apdu_tpci_clean_hex));
            out.push_str(&format!("        hex: '{}',\n", tpdu.apdu.tpci.hex.to_lowercase()));
            out.push_str(&format!("        dataOrControlFlag: '{}',\n", tpdu.apdu.tpci.data_or_control_flag));
            out.push_str(&format!("        numbered: {},\n", tpdu.apdu.tpci.numbered));
            out.push_str(&format!("        sequenceNumber: {},\n", tpdu.apdu.tpci.sequence_number));
            out.push_str(&format!("        firstTwoBitsFromAPCI: {},\n", tpdu.apdu.tpci.first_two_bits_from_apci));
            out.push_str(&format!("        TPCIType: '{}'\n", tpdu.apdu.tpci.tpci_type));
            out.push_str("      },\n");

            out.push_str("      apci: {\n");
            out.push_str(&format!("        obj: '{}',\n", tpdu.apdu.apci.obj));
            out.push_str(&format!("        command: '{}',\n", tpdu.apdu.apci.command));
            out.push_str(&format!("        value: '{}'\n", tpdu.apdu.apci.value));
            out.push_str("      },\n");

            let data_hex: Vec<String> = tpdu.apdu.data.iter().map(|b| format!("{:02x}", b)).collect();
            out.push_str(&format!("      data: <Buffer {}>\n", data_hex.join(" ")));
            out.push_str("    }\n");

            out.push_str("  }\n");
        }

        if out.ends_with(",\n") {
            out.truncate(out.len() - 2);
            out.push('\n');
        }
        out.push('}');
        out
    }
}

#[derive(Debug, Clone)]
pub struct AdditionalInfoDescription {
    pub obj: &'static str,
    pub items: Vec<String>,
}
