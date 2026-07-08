pub struct CemiMessageCodes;
impl CemiMessageCodes {
    pub const L_BUSMON_IND: u8 = 0x2B;
    pub const L_DATA_REQ: u8 = 0x11;
    pub const L_DATA_CON: u8 = 0x2E;
    pub const L_DATA_IND: u8 = 0x29;
    pub const L_RAW_REQ: u8 = 0x10;
    pub const L_RAW_IND: u8 = 0x2D;
    pub const L_RAW_CON: u8 = 0x2F;
    pub const L_POLL_DATA_REQ: u8 = 0x13;
    pub const L_POLL_DATA_CON: u8 = 0x25;
    pub const T_DATA_CONNECTED_REQ: u8 = 0x41;
    pub const T_DATA_CONNECTED_IND: u8 = 0x89;
    pub const T_DATA_INDIVIDUAL_REQ: u8 = 0x4A;
    pub const T_DATA_INDIVIDUAL_IND: u8 = 0x94;
    pub const M_PROP_READ_REQ: u8 = 0xFC;
    pub const M_PROP_READ_CON: u8 = 0xFB;
    pub const M_PROP_WRITE_REQ: u8 = 0xF6;
    pub const M_PROP_WRITE_CON: u8 = 0xF5;
    pub const M_PROP_INFO_IND: u8 = 0xF7;
    pub const M_FUNC_PROP_COMMAND_REQ: u8 = 0xF8;
    pub const M_FUNC_PROP_STATE_READ_REQ: u8 = 0xF9;
    pub const M_FUNC_PROP_COMMAND_CON: u8 = 0xFA;
    pub const M_RESET_REQ: u8 = 0xF1;
    pub const M_RESET_IND: u8 = 0xF0;
}

pub struct EmiMessageCodes;
impl EmiMessageCodes {
    pub const L_BUSMON_IND: u8 = 0x2B;
    pub const L_DATA_REQ: u8 = 0x11;
    pub const L_DATA_CON: u8 = 0x2E;
    pub const L_DATA_IND: u8 = 0x29;
    pub const L_RAW_REQ: u8 = 0x10;
    pub const L_RAW_IND: u8 = 0x2D;
    pub const L_RAW_CON: u8 = 0x2F;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultDestination {
    DLL, // Data Link Layer
    NL,  // Network Layer
    TL,  // Transport Layer
    TLG, // Transport Layer Group Oriented
    TLC, // Transport Layer Connection Oriented
    TLL, // Transport Layer Local
    AL,  // Application Layer
    ALG, // Group Oriented Part of the AL
    MAN, // Management Part of the AL
    PEI, // Physical External Interface
    USR, // USR
    CemiServer,
    CemiClient,
}

#[derive(Debug, Clone, Copy)]
pub struct MessageCodeStandardInfo {
    pub value: u8,
    pub destinations: &'static [DefaultDestination],
}

#[derive(Debug, Clone, Copy)]
pub struct MessageCodeEntry {
    pub service_name: &'static str,
    pub imi1: Option<MessageCodeStandardInfo>,
    pub emi1: Option<MessageCodeStandardInfo>,
    pub emi2_imi2: Option<MessageCodeStandardInfo>,
    pub cemi: Option<MessageCodeStandardInfo>,
}

pub const MESSAGE_CODE_FIELD: &[MessageCodeEntry] = &[
    MessageCodeEntry {
        service_name: "Ph_Data.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x01,
            destinations: &[],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "Ph_data.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x1e,
            destinations: &[],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "Ph_Data.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x19,
            destinations: &[],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "L_Busmon.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x29,
            destinations: &[DefaultDestination::PEI],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x49,
            destinations: &[DefaultDestination::PEI],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x2b,
            destinations: &[DefaultDestination::PEI],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x2b,
            destinations: &[DefaultDestination::PEI],
        }),
    },
    MessageCodeEntry {
        service_name: "L_Data.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x11,
            destinations: &[DefaultDestination::DLL],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x11,
            destinations: &[DefaultDestination::DLL],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x11,
            destinations: &[DefaultDestination::DLL],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x11,
            destinations: &[DefaultDestination::DLL, DefaultDestination::CemiServer],
        }),
    },
    MessageCodeEntry {
        service_name: "L_Data.con",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x2e,
            destinations: &[DefaultDestination::TL],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4e,
            destinations: &[DefaultDestination::TL],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x2e,
            destinations: &[DefaultDestination::NL],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x2e,
            destinations: &[DefaultDestination::NL, DefaultDestination::CemiClient],
        }),
    },
    MessageCodeEntry {
        service_name: "L_Data.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x29,
            destinations: &[DefaultDestination::TL],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x49,
            destinations: &[DefaultDestination::TL],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x29,
            destinations: &[DefaultDestination::NL],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x29,
            destinations: &[DefaultDestination::NL, DefaultDestination::CemiClient],
        }),
    },
    MessageCodeEntry {
        service_name: "L_SystemBroadcast.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x15,
            destinations: &[],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x15,
            destinations: &[],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x17,
            destinations: &[],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "L_SystemBroadcast.con",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x2c,
            destinations: &[],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4c,
            destinations: &[],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x26,
            destinations: &[],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "L_SystemBroadcast.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x2b,
            destinations: &[],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4b,
            destinations: &[],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x28,
            destinations: &[],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "L_Plain_Data.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x10,
            destinations: &[DefaultDestination::DLL],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "L_Raw.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x10,
            destinations: &[DefaultDestination::DLL],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x10,
            destinations: &[DefaultDestination::DLL],
        }),
    },
    MessageCodeEntry {
        service_name: "L_Raw.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0x2d,
            destinations: &[DefaultDestination::NL],
        }),
    },
    MessageCodeEntry {
        service_name: "L_Raw.con",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0x2f,
            destinations: &[DefaultDestination::NL],
        }),
    },
    MessageCodeEntry {
        service_name: "L_Poll_Data.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x13,
            destinations: &[DefaultDestination::DLL],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x13,
            destinations: &[DefaultDestination::DLL, DefaultDestination::CemiServer],
        }),
    },
    MessageCodeEntry {
        service_name: "L_Poll_Data.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x25,
            destinations: &[DefaultDestination::NL],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x25,
            destinations: &[DefaultDestination::NL, DefaultDestination::CemiClient],
        }),
    },
    MessageCodeEntry {
        service_name: "L_Meter.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x24,
            destinations: &[DefaultDestination::NL],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Data_Individual.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x21,
            destinations: &[DefaultDestination::NL],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Data_Individual.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x4e,
            destinations: &[DefaultDestination::TLC],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Data_Individual.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x49,
            destinations: &[DefaultDestination::TLC],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Data_Group.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x22,
            destinations: &[DefaultDestination::NL],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Data_Group.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x3e,
            destinations: &[DefaultDestination::TLG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Data_Group.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x3a,
            destinations: &[DefaultDestination::TLG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Data_Broadcast.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x2c,
            destinations: &[DefaultDestination::NL],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Data_Broadcast.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x4f,
            destinations: &[DefaultDestination::TLC],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Data_Broadcast.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x4d,
            destinations: &[DefaultDestination::TLC],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Poll_Data.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x23,
            destinations: &[DefaultDestination::NL],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "N_Poll_Data.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x35,
            destinations: &[DefaultDestination::TLG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Connect.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x23,
            destinations: &[DefaultDestination::TL],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x23,
            destinations: &[DefaultDestination::TL],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x43,
            destinations: &[DefaultDestination::TLC],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Connect.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x86,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Connect.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x33,
            destinations: &[DefaultDestination::MAN],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x43,
            destinations: &[DefaultDestination::MAN],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x85,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Disconnect.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x24,
            destinations: &[DefaultDestination::TL],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x24,
            destinations: &[DefaultDestination::TL],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x44,
            destinations: &[DefaultDestination::TLC],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Disconnect.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x88,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Disconnect.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x34,
            destinations: &[DefaultDestination::MAN],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x44,
            destinations: &[DefaultDestination::MAN],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x87,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_Connected.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x21,
            destinations: &[DefaultDestination::TL],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x21,
            destinations: &[DefaultDestination::TL],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x41,
            destinations: &[DefaultDestination::TLC],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x41,
            destinations: &[DefaultDestination::CemiServer],
        }),
    },
    MessageCodeEntry {
        service_name: "T_Data_Connected.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x8e,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_Connected.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x39,
            destinations: &[DefaultDestination::MAN],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x49,
            destinations: &[DefaultDestination::MAN],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x89,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x89,
            destinations: &[DefaultDestination::CemiClient],
        }),
    },
    MessageCodeEntry {
        service_name: "T_Data_Group.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x22,
            destinations: &[DefaultDestination::TL],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x22,
            destinations: &[DefaultDestination::TL],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x32,
            destinations: &[DefaultDestination::TLG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_Group.con",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x3e,
            destinations: &[DefaultDestination::ALG],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4e,
            destinations: &[DefaultDestination::ALG],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x7e,
            destinations: &[DefaultDestination::ALG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_Group.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x3a,
            destinations: &[DefaultDestination::ALG],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4a,
            destinations: &[DefaultDestination::ALG],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x7a,
            destinations: &[DefaultDestination::ALG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_Broadcast.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x2b,
            destinations: &[DefaultDestination::TLC],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x2b,
            destinations: &[DefaultDestination::TLC],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x4c,
            destinations: &[DefaultDestination::TLC],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_Broadcast.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x8f,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_Broadcast.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x38,
            destinations: &[DefaultDestination::MAN],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x48,
            destinations: &[DefaultDestination::MAN],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x8d,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_SystemBroadcast.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x25,
            destinations: &[],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x25,
            destinations: &[],
        }),
        emi2_imi2: None,
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_SystemBroadcast.con",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x3c,
            destinations: &[],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4c,
            destinations: &[],
        }),
        emi2_imi2: None,
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_SystemBroadcast.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x3d,
            destinations: &[],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4d,
            destinations: &[],
        }),
        emi2_imi2: None,
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_Individual.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x2a,
            destinations: &[DefaultDestination::TL],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x2a,
            destinations: &[DefaultDestination::TL],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x4a,
            destinations: &[DefaultDestination::TLC],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x4a,
            destinations: &[DefaultDestination::CemiServer],
        }),
    },
    MessageCodeEntry {
        service_name: "T_Data_Individual.con",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x3f,
            destinations: &[DefaultDestination::MAN],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4f,
            destinations: &[DefaultDestination::MAN],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x9c,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Data_Individual.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x32,
            destinations: &[DefaultDestination::MAN],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x42,
            destinations: &[DefaultDestination::MAN],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x94,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: Some(MessageCodeStandardInfo {
            value: 0x94,
            destinations: &[DefaultDestination::CemiClient],
        }),
    },
    MessageCodeEntry {
        service_name: "T_Poll_Data.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x33,
            destinations: &[DefaultDestination::TLG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "T_Poll_Data.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x75,
            destinations: &[DefaultDestination::TLG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_Connect.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xd5,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_Disconnect.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xd7,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_User_Data_Connected.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x31,
            destinations: &[DefaultDestination::MAN],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x31,
            destinations: &[DefaultDestination::MAN],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x82,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_User_Data_Connected.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xd1,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_User_Data_Connected.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x59,
            destinations: &[DefaultDestination::MAN],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x49,
            destinations: &[DefaultDestination::MAN],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xd2,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "A_Data_Group.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x72,
            destinations: &[DefaultDestination::ALG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "A_Data_Group.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xee,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "A_Data_Group.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xea,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_User_Data_Individual.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x81,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_User_Data_Individual.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xde,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_User_Data_Individual.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xd9,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "A_Poll_Data.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x73,
            destinations: &[DefaultDestination::ALG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "A_Poll_Data.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xe5,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_InterfaceObj_Data.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x9a,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_InterfaceObj_Data.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xdc,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_InterfaceObj_Data.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xd4,
            destinations: &[DefaultDestination::USR, DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "U_Value_Read.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x35,
            destinations: &[DefaultDestination::ALG],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x35,
            destinations: &[DefaultDestination::ALG],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x74,
            destinations: &[DefaultDestination::ALG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "U_Value_Read.con",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x55,
            destinations: &[DefaultDestination::USR],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x45,
            destinations: &[DefaultDestination::USR],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xe4,
            destinations: &[DefaultDestination::USR],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "U_Flags_Read.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x37,
            destinations: &[DefaultDestination::ALG],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x37,
            destinations: &[DefaultDestination::ALG],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x7c,
            destinations: &[DefaultDestination::ALG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "U_Flags_Read.con",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x57,
            destinations: &[DefaultDestination::USR],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x47,
            destinations: &[DefaultDestination::USR],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xec,
            destinations: &[DefaultDestination::USR],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "U_Event.ind",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x5d,
            destinations: &[DefaultDestination::USR],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4d,
            destinations: &[DefaultDestination::USR],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xe7,
            destinations: &[DefaultDestination::USR],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "U_Value_Write.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x36,
            destinations: &[DefaultDestination::ALG],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x36,
            destinations: &[DefaultDestination::ALG],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0x71,
            destinations: &[DefaultDestination::ALG],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "U_User_Data",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xd0,
            destinations: &[DefaultDestination::USR],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "PC_Set_Value.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x46,
            destinations: &[DefaultDestination::MAN],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x46,
            destinations: &[DefaultDestination::MAN],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xa6,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "PC_Get_Value.req",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x4c,
            destinations: &[DefaultDestination::MAN],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4c,
            destinations: &[DefaultDestination::MAN],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xac,
            destinations: &[DefaultDestination::MAN],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "PC_Get_Value.con",
        imi1: Some(MessageCodeStandardInfo {
            value: 0x4b,
            destinations: &[DefaultDestination::PEI],
        }),
        emi1: Some(MessageCodeStandardInfo {
            value: 0x4b,
            destinations: &[DefaultDestination::PEI],
        }),
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xab,
            destinations: &[DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "PEI_Identify.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xa7,
            destinations: &[],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "PEI_Identify.con",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xa8,
            destinations: &[DefaultDestination::PEI],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "PEI_Switch.req",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xa9,
            destinations: &[],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "TM_Timer.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: Some(MessageCodeStandardInfo {
            value: 0xc1,
            destinations: &[DefaultDestination::USR],
        }),
        cemi: None,
    },
    MessageCodeEntry {
        service_name: "M_PropRead.req",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xfc,
            destinations: &[DefaultDestination::CemiServer],
        }),
    },
    MessageCodeEntry {
        service_name: "M_PropRead.con",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xfb,
            destinations: &[DefaultDestination::CemiClient],
        }),
    },
    MessageCodeEntry {
        service_name: "M_PropWrite.req",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xf6,
            destinations: &[DefaultDestination::CemiServer],
        }),
    },
    MessageCodeEntry {
        service_name: "M_PropWrite.con",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xf5,
            destinations: &[DefaultDestination::CemiClient],
        }),
    },
    MessageCodeEntry {
        service_name: "M_PropInfo.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xf7,
            destinations: &[DefaultDestination::CemiClient],
        }),
    },
    MessageCodeEntry {
        service_name: "M_FuncPropCommand.req",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xf8,
            destinations: &[DefaultDestination::CemiServer],
        }),
    },
    MessageCodeEntry {
        service_name: "M_FuncPropStateRead.req",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xf9,
            destinations: &[DefaultDestination::CemiServer],
        }),
    },
    MessageCodeEntry {
        service_name: "M_FuncPropCommand.con",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xfa,
            destinations: &[DefaultDestination::CemiClient],
        }),
    },
    MessageCodeEntry {
        service_name: "M_FuncPropStateRead.con",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xfa,
            destinations: &[DefaultDestination::CemiClient],
        }),
    },
    MessageCodeEntry {
        service_name: "M_Reset.req",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xf1,
            destinations: &[DefaultDestination::CemiServer],
        }),
    },
    MessageCodeEntry {
        service_name: "M_Reset.ind",
        imi1: None,
        emi1: None,
        emi2_imi2: None,
        cemi: Some(MessageCodeStandardInfo {
            value: 0xf0,
            destinations: &[DefaultDestination::CemiClient],
        }),
    },
];

pub fn get_cemi_message_code(service_name: &str) -> Option<u8> {
    MESSAGE_CODE_FIELD
        .iter()
        .find(|entry| entry.service_name == service_name)
        .and_then(|entry| entry.cemi)
        .map(|info| info.value)
}

pub fn get_service_name_by_cemi_value(value: u8) -> Option<&'static str> {
    MESSAGE_CODE_FIELD
        .iter()
        .find(|entry| entry.cemi.map_or(false, |info| info.value == value))
        .map(|entry| entry.service_name)
}

pub fn get_emi_message_code(service_name: &str) -> Option<u8> {
    MESSAGE_CODE_FIELD
        .iter()
        .find(|entry| entry.service_name == service_name)
        .and_then(|entry| entry.emi2_imi2)
        .map(|info| info.value)
}

pub fn get_service_name_by_emi_value(value: u8) -> Option<&'static str> {
    MESSAGE_CODE_FIELD
        .iter()
        .find(|entry| entry.emi2_imi2.map_or(false, |info| info.value == value))
        .map(|entry| entry.service_name)
}
