use crate::core::control_field::ControlField;
use crate::core::layers::data::npdu::Npdu;
use crate::core::layers::data::tpdu::Tpdu;
use crate::core::layers::data::apdu::Apdu;
use crate::core::message_code_field::{get_emi_message_code, get_service_name_by_emi_value};
use crate::utils::knx_helper::KnxHelper;
use crate::errors::KnxError;
use byteorder::{BigEndian, ByteOrder};

// ==========================================
// EMI Specific Structures
// ==========================================

#[derive(Debug, Clone)]
pub struct PeiSwitchEmi {
    pub message_code: u8,
    pub control: u8,
}

#[derive(Debug, Clone)]
pub struct LBusmonEmi {
    pub message_code: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LRawEmi {
    pub message_code: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LPlainDataEmi {
    pub message_code: u8,
    pub time: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LDataEmi {
    pub message_code: u8,
    pub control_field1: ControlField,
    pub source_address: String,
    pub destination_address: String,
    pub npdu: Npdu,
}

#[derive(Debug, Clone)]
pub struct LPollDataEmi {
    pub message_code: u8,
    pub control_field1: ControlField,
    pub polling_group: u16,
    pub nr_of_slots: u8,
}

#[derive(Debug, Clone)]
pub struct NDataEmi {
    pub message_code: u8,
    pub control_field1: ControlField,
    pub source_address: String,
    pub destination_address: String,
    pub hop_count: u8,
    pub tpdu: Tpdu,
}

#[derive(Debug, Clone)]
pub struct NPollDataEmi {
    pub message_code: u8,
    pub control_field1: ControlField,
    pub polling_group: u16,
    pub nr_of_slots: u8,
}

#[derive(Debug, Clone)]
pub struct TConnectDisconnectEmi {
    pub message_code: u8,
    pub control: u8,
    pub address: String,
}

#[derive(Debug, Clone)]
pub struct TDataEmi {
    pub message_code: u8,
    pub control_field1: ControlField,
    pub source_address: String,
    pub destination_address: String,
    pub sap: u8,
    pub hop_count: u8,
    pub apdu: Apdu,
}

#[derive(Debug, Clone)]
pub struct TPollDataEmi {
    pub message_code: u8,
    pub control_field1: ControlField,
    pub polling_group: u16,
    pub nr_of_slots: u8,
}

#[derive(Debug, Clone)]
pub struct GenericEmiMessage {
    pub message_code: u8,
    pub data: Vec<u8>,
}

// ==========================================
// Emi Enum Wrapper
// ==========================================
#[derive(Debug, Clone)]
pub enum Emi {
    PeiSwitchReq(PeiSwitchEmi),
    LBusmonInd(LBusmonEmi),
    LRawReq(LRawEmi),
    LRawCon(LRawEmi),
    LRawInd(LRawEmi),
    LPlainDataReq(LPlainDataEmi),
    LDataReq(LDataEmi),
    LDataCon(LDataEmi),
    LDataInd(LDataEmi),
    LPollDataReq(LPollDataEmi),
    LPollDataCon(LPollDataEmi),
    LSystemBroadcastReq(LDataEmi),
    LSystemBroadcastCon(LDataEmi),
    LSystemBroadcastInd(LDataEmi),

    NDataIndividualReq(NDataEmi),
    NDataIndividualCon(NDataEmi),
    NDataIndividualInd(NDataEmi),
    NDataGroupReq(NDataEmi),
    NDataGroupCon(NDataEmi),
    NDataGroupInd(NDataEmi),
    NDataBroadcastReq(NDataEmi),
    NDataBroadcastCon(NDataEmi),
    NDataBroadcastInd(NDataEmi),
    NPollDataReq(NPollDataEmi),
    NPollDataCon(NPollDataEmi),

    TConnectReq(TConnectDisconnectEmi),
    TConnectCon(TConnectDisconnectEmi),
    TConnectInd(TConnectDisconnectEmi),
    TDisconnectReq(TConnectDisconnectEmi),
    TDisconnectCon(TConnectDisconnectEmi),
    TDisconnectInd(TConnectDisconnectEmi),
    TDataConnectedReq(TDataEmi),
    TDataConnectedCon(TDataEmi),
    TDataConnectedInd(TDataEmi),
    TDataGroupReq(TDataEmi),
    TDataGroupCon(TDataEmi),
    TDataGroupInd(TDataEmi),
    TDataIndividualReq(TDataEmi),
    TDataIndividualCon(TDataEmi),
    TDataIndividualInd(TDataEmi),
    TDataBroadcastReq(TDataEmi),
    TDataBroadcastCon(TDataEmi),
    TDataBroadcastInd(TDataEmi),
    TPollDataReq(TPollDataEmi),
    TPollDataCon(TPollDataEmi),

    MConnectInd(TConnectDisconnectEmi),
    MDisconnectInd(TConnectDisconnectEmi),
    MUserDataConnectedReq(TDataEmi),
    MUserDataConnectedCon(TDataEmi),
    MUserDataConnectedInd(TDataEmi),
    MUserDataIndividualReq(TDataEmi),
    MUserDataIndividualCon(TDataEmi),
    MUserDataIndividualInd(TDataEmi),

    APollDataReq(TPollDataEmi),
    APollDataCon(TPollDataEmi),

    MPropReadReq(crate::core::cemi::MProp),
    MPropReadCon(crate::core::cemi::MPropWithPayload),
    MPropWriteReq(crate::core::cemi::MPropWithPayload),
    MPropWriteCon(crate::core::cemi::MPropWriteConfirm),
    MPropInfoInd(crate::core::cemi::MPropWithPayload),
    MFuncPropCommandReq(crate::core::cemi::MFuncProp),
    MFuncPropCommandCon(crate::core::cemi::MFuncPropWithReturn),
    MFuncPropStateReadReq(crate::core::cemi::MFuncProp),
    MFuncPropStateReadCon(crate::core::cemi::MFuncPropWithReturn),
    MResetReq,
    MResetInd,

    ADataGroupReq(TDataEmi),
    ADataGroupCon(TDataEmi),
    ADataGroupInd(TDataEmi),
    ADataIndividualReq(TDataEmi),
    ADataIndividualCon(TDataEmi),
    ADataIndividualInd(TDataEmi),
    ADataBroadcastReq(TDataEmi),
    ADataBroadcastCon(TDataEmi),
    ADataBroadcastInd(TDataEmi),
    ADataConnectedReq(TDataEmi),
    ADataConnectedCon(TDataEmi),
    ADataConnectedInd(TDataEmi),
    AUserDataConnectedReq(TDataEmi),
    AUserDataConnectedCon(TDataEmi),
    AUserDataConnectedInd(TDataEmi),
    AUserDataUnconnectedReq(TDataEmi),
    AUserDataUnconnectedInd(TDataEmi),

    Generic(GenericEmiMessage),
}

impl Emi {
    pub fn get_message_code(&self) -> u8 {
        let name = match self {
            Emi::PeiSwitchReq(_) => "PEI_Switch.req",
            Emi::LBusmonInd(_) => "L_Busmon.ind",
            Emi::LRawReq(_) => "L_Raw.req",
            Emi::LRawCon(_) => "L_Raw.con",
            Emi::LRawInd(_) => "L_Raw.ind",
            Emi::LPlainDataReq(_) => "L_Plain_Data.req",
            Emi::LDataReq(_) => "L_Data.req",
            Emi::LDataCon(_) => "L_Data.con",
            Emi::LDataInd(_) => "L_Data.ind",
            Emi::LPollDataReq(_) => "L_Poll_Data.req",
            Emi::LPollDataCon(_) => "L_Poll_Data.con",
            Emi::LSystemBroadcastReq(_) => "L_SystemBroadcast.req",
            Emi::LSystemBroadcastCon(_) => "L_SystemBroadcast.con",
            Emi::LSystemBroadcastInd(_) => "L_SystemBroadcast.ind",
            Emi::NDataIndividualReq(_) => "N_Data_Individual.req",
            Emi::NDataIndividualCon(_) => "N_Data_Individual.con",
            Emi::NDataIndividualInd(_) => "N_Data_Individual.ind",
            Emi::NDataGroupReq(_) => "N_Data_Group.req",
            Emi::NDataGroupCon(_) => "N_Data_Group.con",
            Emi::NDataGroupInd(_) => "N_Data_Group.ind",
            Emi::NDataBroadcastReq(_) => "N_Data_Broadcast.req",
            Emi::NDataBroadcastCon(_) => "N_Data_Broadcast.con",
            Emi::NDataBroadcastInd(_) => "N_Data_Broadcast.ind",
            Emi::NPollDataReq(_) => "N_Poll_Data.req",
            Emi::NPollDataCon(_) => "N_Poll_Data.con",
            Emi::TConnectReq(_) => "T_Connect.req",
            Emi::TConnectCon(_) => "T_Connect.con",
            Emi::TConnectInd(_) => "T_Connect.ind",
            Emi::TDisconnectReq(_) => "T_Disconnect.req",
            Emi::TDisconnectCon(_) => "T_Disconnect.con",
            Emi::TDisconnectInd(_) => "T_Disconnect.ind",
            Emi::TDataConnectedReq(_) => "T_Data_Connected.req",
            Emi::TDataConnectedCon(_) => "T_Data_Connected.con",
            Emi::TDataConnectedInd(_) => "T_Data_Connected.ind",
            Emi::TDataGroupReq(_) => "T_Data_Group.req",
            Emi::TDataGroupCon(_) => "T_Data_Group.con",
            Emi::TDataGroupInd(_) => "T_Data_Group.ind",
            Emi::TDataIndividualReq(_) => "T_Data_Individual.req",
            Emi::TDataIndividualCon(_) => "T_Data_Individual.con",
            Emi::TDataIndividualInd(_) => "T_Data_Individual.ind",
            Emi::TDataBroadcastReq(_) => "T_Data_Broadcast.req",
            Emi::TDataBroadcastCon(_) => "T_Data_Broadcast.con",
            Emi::TDataBroadcastInd(_) => "T_Data_Broadcast.ind",
            Emi::TPollDataReq(_) => "T_Poll_Data.req",
            Emi::TPollDataCon(_) => "T_Poll_Data.con",
            Emi::MConnectInd(_) => "M_Connect.ind",
            Emi::MDisconnectInd(_) => "M_Disconnect.ind",
            Emi::MUserDataConnectedReq(_) => "M_User_Data_Connected.req",
            Emi::MUserDataConnectedCon(_) => "M_User_Data_Connected.con",
            Emi::MUserDataConnectedInd(_) => "M_User_Data_Connected.ind",
            Emi::MUserDataIndividualReq(_) => "M_User_Data_Individual.req",
            Emi::MUserDataIndividualCon(_) => "M_User_Data_Individual.con",
            Emi::MUserDataIndividualInd(_) => "M_User_Data_Individual.ind",
            Emi::APollDataReq(_) => "A_Poll_Data.req",
            Emi::APollDataCon(_) => "A_Poll_Data.con",
            Emi::MPropReadReq(_) => "M_PropRead.req",
            Emi::MPropReadCon(_) => "M_PropRead.con",
            Emi::MPropWriteReq(_) => "M_PropWrite.req",
            Emi::MPropWriteCon(_) => "M_PropWrite.con",
            Emi::MPropInfoInd(_) => "M_PropInfo.ind",
            Emi::MFuncPropCommandReq(_) => "M_FuncPropCommand.req",
            Emi::MFuncPropCommandCon(_) => "M_FuncPropCommand.con",
            Emi::MFuncPropStateReadReq(_) => "M_FuncPropStateRead.req",
            Emi::MFuncPropStateReadCon(_) => "M_FuncPropStateRead.con",
            Emi::MResetReq => "M_Reset.req",
            Emi::MResetInd => "M_Reset.ind",
            Emi::ADataGroupReq(_) => "A_Data_Group.req",
            Emi::ADataGroupCon(_) => "A_Data_Group.con",
            Emi::ADataGroupInd(_) => "A_Data_Group.ind",
            Emi::ADataIndividualReq(_) => "A_Data_Individual.req",
            Emi::ADataIndividualCon(_) => "A_Data_Individual.con",
            Emi::ADataIndividualInd(_) => "A_Data_Individual.ind",
            Emi::ADataBroadcastReq(_) => "A_Data_Broadcast.req",
            Emi::ADataBroadcastCon(_) => "A_Data_Broadcast.con",
            Emi::ADataBroadcastInd(_) => "A_Data_Broadcast.ind",
            Emi::ADataConnectedReq(_) => "A_Data_Connected.req",
            Emi::ADataConnectedCon(_) => "A_Data_Connected.con",
            Emi::ADataConnectedInd(_) => "A_Data_Connected.ind",
            Emi::AUserDataConnectedReq(_) => "A_User_Data_Connected.req",
            Emi::AUserDataConnectedCon(_) => "A_User_Data_Connected.con",
            Emi::AUserDataConnectedInd(_) => "A_User_Data_Connected.ind",
            Emi::AUserDataUnconnectedReq(_) => "A_User_Data_Unconnected.req",
            Emi::AUserDataUnconnectedInd(_) => "A_User_Data_Unconnected.ind",
            Emi::Generic(ge) => return ge.message_code,
        };
        get_emi_message_code(name)
            .or_else(|| crate::core::message_code_field::get_cemi_message_code(name))
            .unwrap_or(0)
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        match self {
            Emi::PeiSwitchReq(ps) => {
                vec![self.get_message_code(), ps.control]
            }
            Emi::LBusmonInd(lb) => {
                let mut buffer = vec![0u8; 1 + lb.data.len()];
                buffer[0] = self.get_message_code();
                buffer[1..].copy_from_slice(&lb.data);
                buffer
            }
            Emi::LRawReq(lr) | Emi::LRawCon(lr) | Emi::LRawInd(lr) => {
                let mut buffer = vec![0u8; 1 + lr.data.len()];
                buffer[0] = self.get_message_code();
                buffer[1..].copy_from_slice(&lr.data);
                buffer
            }
            Emi::LPlainDataReq(lp) => {
                let mut buffer = vec![0u8; 6 + lp.data.len()];
                buffer[0] = self.get_message_code();
                buffer[1] = 0x00;
                BigEndian::write_u32(&mut buffer[2..6], lp.time);
                buffer[6..].copy_from_slice(&lp.data);
                buffer
            }
            Emi::LDataReq(ld) | Emi::LDataCon(ld) | Emi::LDataInd(ld) |
            Emi::LSystemBroadcastReq(ld) | Emi::LSystemBroadcastCon(ld) | Emi::LSystemBroadcastInd(ld) => {
                let npdu_buffer = ld.npdu.to_buffer();
                let mut buffer = vec![0u8; 6 + npdu_buffer.len()];
                buffer[0] = self.get_message_code();
                buffer[1] = ld.control_field1.get_buffer()[0];

                if matches!(self, Emi::LDataInd(_) | Emi::LSystemBroadcastInd(_)) {
                    let src_bytes = KnxHelper::get_address_from_string(&ld.source_address).unwrap_or_default();
                    buffer[2..4].copy_from_slice(&src_bytes);
                } else {
                    buffer[2] = 0;
                    buffer[3] = 0;
                }

                let dst_bytes = KnxHelper::get_address_from_string(&ld.destination_address).unwrap_or_default();
                buffer[4..6].copy_from_slice(&dst_bytes);

                buffer[6..].copy_from_slice(&npdu_buffer);
                buffer
            }
            Emi::LPollDataReq(lp) | Emi::LPollDataCon(lp) => {
                let mut buffer = vec![0u8; 7];
                buffer[0] = self.get_message_code();
                buffer[1] = lp.control_field1.get_buffer()[0];
                BigEndian::write_u16(&mut buffer[4..6], lp.polling_group);
                buffer[6] = lp.nr_of_slots & 0x0f;
                buffer
            }
            Emi::NDataIndividualReq(nd) | Emi::NDataIndividualCon(nd) | Emi::NDataIndividualInd(nd) |
            Emi::NDataGroupReq(nd) | Emi::NDataGroupCon(nd) | Emi::NDataGroupInd(nd) |
            Emi::NDataBroadcastReq(nd) | Emi::NDataBroadcastCon(nd) | Emi::NDataBroadcastInd(nd) => {
                let tpdu_buffer = nd.tpdu.to_buffer();
                let mut buffer = vec![0u8; 7 + tpdu_buffer.len()];
                buffer[0] = self.get_message_code();
                buffer[1] = nd.control_field1.get_buffer()[0];

                if matches!(self, Emi::NDataIndividualInd(_) | Emi::NDataGroupInd(_) | Emi::NDataBroadcastInd(_)) {
                    let src_bytes = KnxHelper::get_address_from_string(&nd.source_address).unwrap_or_default();
                    buffer[2..4].copy_from_slice(&src_bytes);
                } else {
                    buffer[2] = 0;
                    buffer[3] = 0;
                }

                let dst_bytes = KnxHelper::get_address_from_string(&nd.destination_address).unwrap_or_default();
                buffer[4..6].copy_from_slice(&dst_bytes);

                buffer[6] = ((nd.hop_count & 0x0f) << 4) | (tpdu_buffer.len() as u8 & 0x0f);
                buffer[7..].copy_from_slice(&tpdu_buffer);
                buffer
            }
            Emi::NPollDataReq(np) | Emi::NPollDataCon(np) => {
                let mut buffer = vec![0u8; 7];
                buffer[0] = self.get_message_code();
                buffer[1] = np.control_field1.get_buffer()[0];
                BigEndian::write_u16(&mut buffer[4..6], np.polling_group);
                buffer[6] = np.nr_of_slots & 0x0f;
                buffer
            }
            Emi::TConnectReq(tc) | Emi::TDisconnectReq(tc) => {
                let mut buffer = vec![0u8; 6];
                buffer[0] = self.get_message_code();
                buffer[1] = tc.control;
                let addr_bytes = KnxHelper::get_address_from_string(&tc.address).unwrap_or_default();
                buffer[4..6].copy_from_slice(&addr_bytes);
                buffer
            }
            Emi::TConnectCon(tc) | Emi::TConnectInd(tc) |
            Emi::TDisconnectCon(tc) | Emi::TDisconnectInd(tc) |
            Emi::MConnectInd(tc) | Emi::MDisconnectInd(tc) => {
                let mut buffer = vec![0u8; 6];
                buffer[0] = self.get_message_code();
                buffer[1] = tc.control;
                let addr_bytes = KnxHelper::get_address_from_string(&tc.address).unwrap_or_default();
                buffer[2..4].copy_from_slice(&addr_bytes);
                buffer
            }
            Emi::TDataConnectedReq(td) | Emi::TDataConnectedCon(td) | Emi::TDataConnectedInd(td) |
            Emi::TDataGroupReq(td) | Emi::TDataGroupCon(td) | Emi::TDataGroupInd(td) |
            Emi::TDataIndividualReq(td) | Emi::TDataIndividualCon(td) | Emi::TDataIndividualInd(td) |
            Emi::TDataBroadcastReq(td) | Emi::TDataBroadcastCon(td) | Emi::TDataBroadcastInd(td) |
            Emi::MUserDataConnectedReq(td) | Emi::MUserDataConnectedCon(td) | Emi::MUserDataConnectedInd(td) |
            Emi::MUserDataIndividualReq(td) | Emi::MUserDataIndividualCon(td) | Emi::MUserDataIndividualInd(td) |
            Emi::ADataGroupReq(td) | Emi::ADataGroupCon(td) | Emi::ADataGroupInd(td) |
            Emi::ADataIndividualReq(td) | Emi::ADataIndividualCon(td) | Emi::ADataIndividualInd(td) |
            Emi::ADataBroadcastReq(td) | Emi::ADataBroadcastCon(td) | Emi::ADataBroadcastInd(td) |
            Emi::ADataConnectedReq(td) | Emi::ADataConnectedCon(td) | Emi::ADataConnectedInd(td) |
            Emi::AUserDataConnectedReq(td) | Emi::AUserDataConnectedCon(td) | Emi::AUserDataConnectedInd(td) |
            Emi::AUserDataUnconnectedReq(td) | Emi::AUserDataUnconnectedInd(td) => {
                let apdu_buffer = td.apdu.to_buffer();
                let mut buffer = vec![0u8; 7 + apdu_buffer.len()];
                buffer[0] = self.get_message_code();
                buffer[1] = td.control_field1.get_buffer()[0];

                let is_ind = self.is_ind_variant();
                if is_ind {
                    let src_bytes = KnxHelper::get_address_from_string(&td.source_address).unwrap_or_default();
                    buffer[2..4].copy_from_slice(&src_bytes);
                } else if !self.is_con_variant() {
                    let dst_bytes = KnxHelper::get_address_from_string(&td.destination_address).unwrap_or_default();
                    buffer[4..6].copy_from_slice(&dst_bytes);
                }

                buffer[5] = td.sap;
                buffer[6] = ((td.hop_count & 0x07) << 4) | (apdu_buffer.len() as u8 & 0x0f);
                buffer[7..].copy_from_slice(&apdu_buffer);
                buffer
            }
            Emi::TPollDataReq(tp) | Emi::TPollDataCon(tp) |
            Emi::APollDataReq(tp) | Emi::APollDataCon(tp) => {
                let mut buffer = vec![0u8; 7];
                buffer[0] = self.get_message_code();
                buffer[1] = tp.control_field1.get_buffer()[0];
                BigEndian::write_u16(&mut buffer[4..6], tp.polling_group);
                buffer[6] = tp.nr_of_slots & 0x0f;
                buffer
            }
            Emi::MPropReadReq(mp) => {
                let mut buffer = vec![0u8; 7];
                buffer[0] = self.get_message_code();
                BigEndian::write_u16(&mut buffer[1..3], mp.interface_object_type);
                buffer[3] = mp.object_instance;
                buffer[4] = mp.property_id;
                let val5 = (mp.start_index & 0x0FFF) | ((mp.number_of_elements as u16 & 0x0F) << 12);
                BigEndian::write_u16(&mut buffer[5..7], val5);
                buffer
            }
            Emi::MPropReadCon(mp) | Emi::MPropWriteReq(mp) | Emi::MPropInfoInd(mp) => {
                let mut buffer = vec![0u8; 7 + mp.data.len()];
                buffer[0] = self.get_message_code();
                BigEndian::write_u16(&mut buffer[1..3], mp.interface_object_type);
                buffer[3] = mp.object_instance;
                buffer[4] = mp.property_id;
                let val5 = (mp.start_index & 0x0FFF) | ((mp.number_of_elements as u16 & 0x0F) << 12);
                BigEndian::write_u16(&mut buffer[5..7], val5);
                buffer[7..].copy_from_slice(&mp.data);
                buffer
            }
            Emi::MPropWriteCon(mp) => {
                let size = if mp.number_of_elements == 0 { 8 } else { 7 };
                let mut buffer = vec![0u8; size];
                buffer[0] = self.get_message_code();
                BigEndian::write_u16(&mut buffer[1..3], mp.interface_object_type);
                buffer[3] = mp.object_instance;
                buffer[4] = mp.property_id;
                let val5 = (mp.start_index & 0x0FFF) | ((mp.number_of_elements as u16 & 0x0F) << 12);
                BigEndian::write_u16(&mut buffer[5..7], val5);
                if mp.number_of_elements == 0 {
                    buffer[7] = mp.error_info;
                }
                buffer
            }
            Emi::MFuncPropCommandReq(mf) | Emi::MFuncPropStateReadReq(mf) => {
                let mut buffer = vec![0u8; 5 + mf.data.len()];
                buffer[0] = self.get_message_code();
                BigEndian::write_u16(&mut buffer[1..3], mf.interface_object_type);
                buffer[3] = mf.object_instance;
                buffer[4] = mf.property_id;
                buffer[5..].copy_from_slice(&mf.data);
                buffer
            }
            Emi::MFuncPropCommandCon(mf) | Emi::MFuncPropStateReadCon(mf) => {
                let mut buffer = vec![0u8; 6 + mf.data.len()];
                buffer[0] = self.get_message_code();
                BigEndian::write_u16(&mut buffer[1..3], mf.interface_object_type);
                buffer[3] = mf.object_instance;
                buffer[4] = mf.property_id;
                buffer[5] = mf.return_code;
                buffer[6..].copy_from_slice(&mf.data);
                buffer
            }
            Emi::MResetReq | Emi::MResetInd => {
                vec![self.get_message_code()]
            }
            Emi::Generic(ge) => {
                let mut buffer = vec![0u8; 1 + ge.data.len()];
                buffer[0] = ge.message_code;
                buffer[1..].copy_from_slice(&ge.data);
                buffer
            }
        }
    }

    fn is_ind_variant(&self) -> bool {
        let name = get_service_name_by_emi_value(self.get_message_code()).unwrap_or("");
        name.ends_with(".ind")
    }

    fn is_con_variant(&self) -> bool {
        let name = get_service_name_by_emi_value(self.get_message_code()).unwrap_or("");
        name.ends_with(".con")
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.is_empty() {
            return Err(KnxError::InvalidParametersForDpt);
        }

        let message_code = buffer[0];
        let service_name = get_service_name_by_emi_value(message_code)
            .or_else(|| crate::core::message_code_field::get_service_name_by_cemi_value(message_code))
            .unwrap_or("");

        match service_name {
            "PEI_Switch.req" => {
                if buffer.len() < 2 { return Err(KnxError::InvalidParametersForDpt); }
                Ok(Emi::PeiSwitchReq(PeiSwitchEmi { message_code, control: buffer[1] }))
            }
            "L_Busmon.ind" => {
                Ok(Emi::LBusmonInd(LBusmonEmi { message_code, data: buffer[1..].to_vec() }))
            }
            "L_Raw.req" | "L_Raw.con" | "L_Raw.ind" => {
                let data = buffer[1..].to_vec();
                let lr = LRawEmi { message_code, data };
                if service_name == "L_Raw.req" {
                    Ok(Emi::LRawReq(lr))
                } else if service_name == "L_Raw.con" {
                    Ok(Emi::LRawCon(lr))
                } else {
                    Ok(Emi::LRawInd(lr))
                }
            }
            "L_Plain_Data.req" => {
                if buffer.len() < 6 { return Err(KnxError::InvalidParametersForDpt); }
                let time = BigEndian::read_u32(&buffer[2..6]);
                Ok(Emi::LPlainDataReq(LPlainDataEmi { message_code, time, data: buffer[6..].to_vec() }))
            }
            "L_Data.req" | "L_Data.con" | "L_Data.ind" |
            "L_SystemBroadcast.req" | "L_SystemBroadcast.con" | "L_SystemBroadcast.ind" => {
                if buffer.len() < 7 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let control_field1 = ControlField::new(buffer[1]);
                let is_ind = service_name.ends_with(".ind");
                let source_address = if is_ind {
                    KnxHelper::get_address_to_string(&buffer[2..4], ".", false)?
                } else {
                    "0.0.0".to_string()
                };

                let npdu = Npdu::from_buffer(&buffer[6..])?;
                let is_group = npdu.address_type == crate::core::control_field_extended::AddressType::Group;
                let destination_address = KnxHelper::get_address_to_string(
                    &buffer[4..6],
                    if is_group { "/" } else { "." },
                    is_group
                )?;

                let ld = LDataEmi {
                    message_code,
                    control_field1,
                    source_address,
                    destination_address,
                    npdu,
                };

                match service_name {
                    "L_Data.req" => Ok(Emi::LDataReq(ld)),
                    "L_Data.con" => Ok(Emi::LDataCon(ld)),
                    "L_Data.ind" => Ok(Emi::LDataInd(ld)),
                    "L_SystemBroadcast.req" => Ok(Emi::LSystemBroadcastReq(ld)),
                    "L_SystemBroadcast.con" => Ok(Emi::LSystemBroadcastCon(ld)),
                    _ => Ok(Emi::LSystemBroadcastInd(ld)),
                }
            }
            "L_Poll_Data.req" | "L_Poll_Data.con" => {
                if buffer.len() < 7 { return Err(KnxError::InvalidParametersForDpt); }
                let control_field1 = ControlField::new(buffer[1]);
                let polling_group = BigEndian::read_u16(&buffer[4..6]);
                let nr_of_slots = buffer[6] & 0x0f;
                let lp = LPollDataEmi { message_code, control_field1, polling_group, nr_of_slots };
                if service_name == "L_Poll_Data.req" {
                    Ok(Emi::LPollDataReq(lp))
                } else {
                    Ok(Emi::LPollDataCon(lp))
                }
            }
            "N_Data_Individual.req" | "N_Data_Individual.con" | "N_Data_Individual.ind" |
            "N_Data_Group.req" | "N_Data_Group.con" | "N_Data_Group.ind" |
            "N_Data_Broadcast.req" | "N_Data_Broadcast.con" | "N_Data_Broadcast.ind" => {
                if buffer.len() < 7 { return Err(KnxError::InvalidParametersForDpt); }
                let control_field1 = ControlField::new(buffer[1]);
                let is_ind = service_name.ends_with(".ind");
                let source_address = if is_ind {
                    KnxHelper::get_address_to_string(&buffer[2..4], ".", false)?
                } else {
                    "0.0.0".to_string()
                };
                let is_group = service_name.contains("Group") || service_name.contains("Broadcast");
                let destination_address = KnxHelper::get_address_to_string(
                    &buffer[4..6],
                    if is_group { "/" } else { "." },
                    is_group
                )?;
                let octet6 = buffer[6];
                let hop_count = (octet6 >> 4) & 0x0f;
                let length = (octet6 & 0x0f) as usize;
                if buffer.len() < 7 + length { return Err(KnxError::InvalidParametersForDpt); }
                let tpdu = Tpdu::from_buffer(&buffer[7..7+length])?;

                let nd = NDataEmi { message_code, control_field1, source_address, destination_address, hop_count, tpdu };
                match service_name {
                    "N_Data_Individual.req" => Ok(Emi::NDataIndividualReq(nd)),
                    "N_Data_Individual.con" => Ok(Emi::NDataIndividualCon(nd)),
                    "N_Data_Individual.ind" => Ok(Emi::NDataIndividualInd(nd)),
                    "N_Data_Group.req" => Ok(Emi::NDataGroupReq(nd)),
                    "N_Data_Group.con" => Ok(Emi::NDataGroupCon(nd)),
                    "N_Data_Group.ind" => Ok(Emi::NDataGroupInd(nd)),
                    "N_Data_Broadcast.req" => Ok(Emi::NDataBroadcastReq(nd)),
                    "N_Data_Broadcast.con" => Ok(Emi::NDataBroadcastCon(nd)),
                    _ => Ok(Emi::NDataBroadcastInd(nd)),
                }
            }
            "N_Poll_Data.req" | "N_Poll_Data.con" => {
                if buffer.len() < 7 { return Err(KnxError::InvalidParametersForDpt); }
                let control_field1 = ControlField::new(buffer[1]);
                let polling_group = BigEndian::read_u16(&buffer[4..6]);
                let nr_of_slots = buffer[6] & 0x0f;
                let np = NPollDataEmi { message_code, control_field1, polling_group, nr_of_slots };
                if service_name == "N_Poll_Data.req" {
                    Ok(Emi::NPollDataReq(np))
                } else {
                    Ok(Emi::NPollDataCon(np))
                }
            }
            "T_Connect.req" | "T_Disconnect.req" => {
                if buffer.len() < 6 { return Err(KnxError::InvalidParametersForDpt); }
                let address = KnxHelper::get_address_to_string(&buffer[4..6], ".", false)?;
                let tc = TConnectDisconnectEmi { message_code, control: buffer[1], address };
                if service_name == "T_Connect.req" {
                    Ok(Emi::TConnectReq(tc))
                } else {
                    Ok(Emi::TDisconnectReq(tc))
                }
            }
            "T_Connect.con" | "T_Connect.ind" | "T_Disconnect.con" | "T_Disconnect.ind" |
            "M_Connect.ind" | "M_Disconnect.ind" => {
                if buffer.len() < 4 { return Err(KnxError::InvalidParametersForDpt); }
                let address = KnxHelper::get_address_to_string(&buffer[2..4], ".", false)?;
                let tc = TConnectDisconnectEmi { message_code, control: buffer[1], address };
                match service_name {
                    "T_Connect.con" => Ok(Emi::TConnectCon(tc)),
                    "T_Connect.ind" => Ok(Emi::TConnectInd(tc)),
                    "T_Disconnect.con" => Ok(Emi::TDisconnectCon(tc)),
                    "T_Disconnect.ind" => Ok(Emi::TDisconnectInd(tc)),
                    "M_Connect.ind" => Ok(Emi::MConnectInd(tc)),
                    _ => Ok(Emi::MDisconnectInd(tc)),
                }
            }
            "T_Data_Connected.req" | "T_Data_Connected.con" | "T_Data_Connected.ind" |
            "T_Data_Group.req" | "T_Data_Group.con" | "T_Data_Group.ind" |
            "T_Data_Individual.req" | "T_Data_Individual.con" | "T_Data_Individual.ind" |
            "T_Data_Broadcast.req" | "T_Data_Broadcast.con" | "T_Data_Broadcast.ind" |
            "M_User_Data_Connected.req" | "M_User_Data_Connected.con" | "M_User_Data_Connected.ind" |
            "M_User_Data_Individual.req" | "M_User_Data_Individual.con" | "M_User_Data_Individual.ind" |
            "A_Data_Group.req" | "A_Data_Group.con" | "A_Data_Group.ind" |
            "A_Data_Individual.req" | "A_Data_Individual.con" | "A_Data_Individual.ind" |
            "A_Data_Broadcast.req" | "A_Data_Broadcast.con" | "A_Data_Broadcast.ind" |
            "A_Data_Connected.req" | "A_Data_Connected.con" | "A_Data_Connected.ind" |
            "A_UserData_Connected.req" | "A_UserData_Connected.con" | "A_UserData_Connected.ind" |
            "A_User_Data_Connected.req" | "A_User_Data_Connected.con" | "A_User_Data_Connected.ind" |
            "A_UserData_Unconnected.req" | "A_UserData_Unconnected.ind" |
            "A_User_Data_Unconnected.req" | "A_User_Data_Unconnected.ind" => {
                if buffer.len() < 7 { return Err(KnxError::InvalidParametersForDpt); }
                let control_field1 = ControlField::new(buffer[1]);
                let is_ind = service_name.ends_with(".ind");
                let is_con = service_name.ends_with(".con");
                let source_address = if is_ind {
                    KnxHelper::get_address_to_string(&buffer[2..4], ".", false)?
                } else {
                    "0.0.0".to_string()
                };
                let is_group = service_name.contains("Group") || service_name.contains("Broadcast");
                let destination_address = if !is_ind && !is_con {
                    KnxHelper::get_address_to_string(
                        &buffer[4..6],
                        if is_group { "/" } else { "." },
                        is_group
                    )?
                } else {
                    "0.0.0".to_string()
                };

                let sap = buffer[5];
                let octet6 = buffer[6];
                let hop_count = (octet6 >> 4) & 0x07;
                let length = (octet6 & 0x0f) as usize;
                if buffer.len() < 7 + length { return Err(KnxError::InvalidParametersForDpt); }
                let apdu = Apdu::from_buffer(&buffer[7..7+length])?;

                let td = TDataEmi { message_code, control_field1, source_address, destination_address, sap, hop_count, apdu };
                match service_name {
                    "T_Data_Connected.req" => Ok(Emi::TDataConnectedReq(td)),
                    "T_Data_Connected.con" => Ok(Emi::TDataConnectedCon(td)),
                    "T_Data_Connected.ind" => Ok(Emi::TDataConnectedInd(td)),
                    "T_Data_Group.req" => Ok(Emi::TDataGroupReq(td)),
                    "T_Data_Group.con" => Ok(Emi::TDataGroupCon(td)),
                    "T_Data_Group.ind" => Ok(Emi::TDataGroupInd(td)),
                    "T_Data_Individual.req" => Ok(Emi::TDataIndividualReq(td)),
                    "T_Data_Individual.con" => Ok(Emi::TDataIndividualCon(td)),
                    "T_Data_Individual.ind" => Ok(Emi::TDataIndividualInd(td)),
                    "T_Data_Broadcast.req" => Ok(Emi::TDataBroadcastReq(td)),
                    "T_Data_Broadcast.con" => Ok(Emi::TDataBroadcastCon(td)),
                    "T_Data_Broadcast.ind" => Ok(Emi::TDataBroadcastInd(td)),
                    "M_User_Data_Connected.req" => Ok(Emi::MUserDataConnectedReq(td)),
                    "M_User_Data_Connected.con" => Ok(Emi::MUserDataConnectedCon(td)),
                    "M_User_Data_Connected.ind" => Ok(Emi::MUserDataConnectedInd(td)),
                    "M_User_Data_Individual.req" => Ok(Emi::MUserDataIndividualReq(td)),
                    "M_User_Data_Individual.con" => Ok(Emi::MUserDataIndividualCon(td)),
                    "M_User_Data_Individual.ind" => Ok(Emi::MUserDataIndividualInd(td)),
                    "A_Data_Group.req" => Ok(Emi::ADataGroupReq(td)),
                    "A_Data_Group.con" => Ok(Emi::ADataGroupCon(td)),
                    "A_Data_Group.ind" => Ok(Emi::ADataGroupInd(td)),
                    "A_Data_Individual.req" => Ok(Emi::ADataIndividualReq(td)),
                    "A_Data_Individual.con" => Ok(Emi::ADataIndividualCon(td)),
                    "A_Data_Individual.ind" => Ok(Emi::ADataIndividualInd(td)),
                    "A_Data_Broadcast.req" => Ok(Emi::ADataBroadcastReq(td)),
                    "A_Data_Broadcast.con" => Ok(Emi::ADataBroadcastCon(td)),
                    "A_Data_Broadcast.ind" => Ok(Emi::ADataBroadcastInd(td)),
                    "A_Data_Connected.req" => Ok(Emi::ADataConnectedReq(td)),
                    "A_Data_Connected.con" => Ok(Emi::ADataConnectedCon(td)),
                    "A_Data_Connected.ind" => Ok(Emi::ADataConnectedInd(td)),
                    "A_UserData_Connected.req" | "A_User_Data_Connected.req" => Ok(Emi::AUserDataConnectedReq(td)),
                    "A_UserData_Connected.con" | "A_User_Data_Connected.con" => Ok(Emi::AUserDataConnectedCon(td)),
                    "A_UserData_Connected.ind" | "A_User_Data_Connected.ind" => Ok(Emi::AUserDataConnectedInd(td)),
                    "A_UserData_Unconnected.req" | "A_User_Data_Unconnected.req" => Ok(Emi::AUserDataUnconnectedReq(td)),
                    _ => Ok(Emi::AUserDataUnconnectedInd(td)),
                }
            }
            "T_Poll_Data.req" | "T_Poll_Data.con" |
            "A_Poll_Data.req" | "A_Poll_Data.con" => {
                if buffer.len() < 7 { return Err(KnxError::InvalidParametersForDpt); }
                let control_field1 = ControlField::new(buffer[1]);
                let polling_group = BigEndian::read_u16(&buffer[4..6]);
                let nr_of_slots = buffer[6] & 0x0f;
                let tp = TPollDataEmi { message_code, control_field1, polling_group, nr_of_slots };
                
                if service_name == "T_Poll_Data.req" || service_name == "A_Poll_Data.req" {
                    if service_name == "T_Poll_Data.req" {
                        Ok(Emi::TPollDataReq(tp))
                    } else {
                        Ok(Emi::APollDataReq(tp))
                    }
                } else {
                    if service_name == "T_Poll_Data.con" {
                        Ok(Emi::TPollDataCon(tp))
                    } else {
                        Ok(Emi::APollDataCon(tp))
                    }
                }
            }
            "M_PropRead.req" => {
                if buffer.len() < 7 { return Err(KnxError::InvalidParametersForDpt); }
                let interface_object_type = BigEndian::read_u16(&buffer[1..3]);
                let object_instance = buffer[3];
                let property_id = buffer[4];
                let val5 = BigEndian::read_u16(&buffer[5..7]);
                let start_index = val5 & 0x0FFF;
                let number_of_elements = ((val5 >> 12) & 0x0F) as u8;
                Ok(Emi::MPropReadReq(crate::core::cemi::MProp {
                    interface_object_type,
                    object_instance,
                    property_id,
                    number_of_elements,
                    start_index,
                }))
            }
            "M_PropRead.con" | "M_PropWrite.req" | "M_PropInfo.ind" => {
                if buffer.len() < 7 { return Err(KnxError::InvalidParametersForDpt); }
                let interface_object_type = BigEndian::read_u16(&buffer[1..3]);
                let object_instance = buffer[3];
                let property_id = buffer[4];
                let val5 = BigEndian::read_u16(&buffer[5..7]);
                let start_index = val5 & 0x0FFF;
                let number_of_elements = ((val5 >> 12) & 0x0F) as u8;
                let data = buffer[7..].to_vec();
                let mp = crate::core::cemi::MPropWithPayload {
                    interface_object_type,
                    object_instance,
                    property_id,
                    number_of_elements,
                    start_index,
                    data,
                };
                match service_name {
                    "M_PropRead.con" => Ok(Emi::MPropReadCon(mp)),
                    "M_PropWrite.req" => Ok(Emi::MPropWriteReq(mp)),
                    _ => Ok(Emi::MPropInfoInd(mp)),
                }
            }
            "M_PropWrite.con" => {
                if buffer.len() < 7 { return Err(KnxError::InvalidParametersForDpt); }
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
                Ok(Emi::MPropWriteCon(crate::core::cemi::MPropWriteConfirm {
                    interface_object_type,
                    object_instance,
                    property_id,
                    number_of_elements,
                    start_index,
                    error_info,
                }))
            }
            "M_FuncPropCommand.req" | "M_FuncPropStateRead.req" => {
                if buffer.len() < 5 { return Err(KnxError::InvalidParametersForDpt); }
                let interface_object_type = BigEndian::read_u16(&buffer[1..3]);
                let object_instance = buffer[3];
                let property_id = buffer[4];
                let data = buffer[5..].to_vec();
                let mf = crate::core::cemi::MFuncProp {
                    interface_object_type,
                    object_instance,
                    property_id,
                    data,
                };
                if service_name == "M_FuncPropCommand.req" {
                    Ok(Emi::MFuncPropCommandReq(mf))
                } else {
                    Ok(Emi::MFuncPropStateReadReq(mf))
                }
            }
            "M_FuncPropCommand.con" | "M_FuncPropStateRead.con" => {
                if buffer.len() < 6 { return Err(KnxError::InvalidParametersForDpt); }
                let interface_object_type = BigEndian::read_u16(&buffer[1..3]);
                let object_instance = buffer[3];
                let property_id = buffer[4];
                let return_code = buffer[5];
                let data = buffer[6..].to_vec();
                let mf = crate::core::cemi::MFuncPropWithReturn {
                    interface_object_type,
                    object_instance,
                    property_id,
                    return_code,
                    data,
                };
                if service_name == "M_FuncPropCommand.con" {
                    Ok(Emi::MFuncPropCommandCon(mf))
                } else {
                    Ok(Emi::MFuncPropStateReadCon(mf))
                }
            }
            "M_Reset.req" => Ok(Emi::MResetReq),
            "M_Reset.ind" => Ok(Emi::MResetInd),
            _ => {
                let data = buffer[1..].to_vec();
                Ok(Emi::Generic(GenericEmiMessage { message_code, data }))
            }
        }
    }
}
