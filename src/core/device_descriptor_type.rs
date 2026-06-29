#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceDescriptorType0 {
    value: u16,
}

impl DeviceDescriptorType0 {
    pub const fn new(value: u16) -> Self {
        Self { value }
    }

    pub fn value(&self) -> u16 {
        self.value
    }

    pub fn mask_type(&self) -> u8 {
        ((self.value >> 8) & 0xFF) as u8
    }

    pub fn medium_type(&self) -> u8 {
        ((self.value >> 12) & 0x0F) as u8
    }

    pub fn firmware_type(&self) -> u8 {
        ((self.value >> 8) & 0x0F) as u8
    }

    pub fn firmware_version(&self) -> u8 {
        (self.value & 0xFF) as u8
    }

    pub fn version(&self) -> u8 {
        ((self.value >> 4) & 0x0F) as u8
    }

    pub fn subcode(&self) -> u8 {
        (self.value & 0x0F) as u8
    }

    pub const TP1_BCU_1_SYSTEM_1_V0: Self = Self::new(0x0010);
    pub const TP1_BCU_1_SYSTEM_1_V1: Self = Self::new(0x0011);
    pub const TP1_BCU_1_SYSTEM_1_V2: Self = Self::new(0x0012);
    pub const TP1_BCU_1_SYSTEM_1_V3: Self = Self::new(0x0013);
    pub const TP1_BCU_2_SYSTEM_2_V0: Self = Self::new(0x0020);
    pub const TP1_BCU_2_SYSTEM_2_V1: Self = Self::new(0x0021);
    pub const TP1_BCU_2_SYSTEM_2_V5: Self = Self::new(0x0025);
    pub const TP1_SYSTEM_300: Self = Self::new(0x0300);
    pub const TP1_USB_INTERFACE_V1: Self = Self::new(0x0310);
    pub const TP1_USB_INTERFACE_V2: Self = Self::new(0x0311);
    pub const TP1_BIM_M112_V0: Self = Self::new(0x0700);
    pub const TP1_BIM_M112_V1: Self = Self::new(0x0701);
    pub const TP1_BIM_M112_V5: Self = Self::new(0x0705);
    pub const TP1_SYSTEM_B: Self = Self::new(0x07B0);
    pub const TP1_IR_DECODER_V0: Self = Self::new(0x0810);
    pub const TP1_IR_DECODER_V1: Self = Self::new(0x0811);
    pub const TP1_COUPLER_1_0: Self = Self::new(0x0910);
    pub const TP1_COUPLER_1_1: Self = Self::new(0x0911);
    pub const TP1_COUPLER_1_2: Self = Self::new(0x0912);
    pub const KNXNET_IP_ROUTER: Self = Self::new(0x091A);
    pub const TP1_NONE_FD: Self = Self::new(0x0AFD);
    pub const TP1_NONE_FE: Self = Self::new(0x0AFE);
    pub const PL110_BCU_1_V2: Self = Self::new(0x1012);
    pub const PL110_BCU_1_V3: Self = Self::new(0x1013);
    pub const PL110_USB_INTERFACE_V1: Self = Self::new(0x1310);
    pub const PL110_USB_INTERFACE_V2: Self = Self::new(0x1311);
    pub const PL110_SYSTEM_B: Self = Self::new(0x17B0);
    pub const TP1_PL110_MEDIA_COUPLER: Self = Self::new(0x1900);
    pub const RF_BI_DIRECTIONAL_DEVICES: Self = Self::new(0x2010);
    pub const RF_UNI_DIRECTIONAL_DEVICES: Self = Self::new(0x2110);
    pub const RF_USB_INTERFACE_V2: Self = Self::new(0x2311);
    pub const TP0_BCU_1_V2: Self = Self::new(0x3012);
    pub const PL132_BCU_1_V2: Self = Self::new(0x4012);
    pub const KNX_IP_SYSTEM_7: Self = Self::new(0x5705);
    pub const KNX_IP_SYSTEM_B: Self = Self::new(0x57B0);
}
