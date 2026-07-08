#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressType {
    Individual = 0,
    Group = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendedFrameFormat {
    PointToPointOrStandardGroupAddressed = 0,
    MulticastZoneAddressed11 = 0b0111,
    MulticastZoneAddressed01 = 0b0101,
    MulticastZoneAddressed10 = 0b0110,
    MulticastZoneAddressed00 = 0b0100,
}

impl ExtendedFrameFormat {
    pub fn from_u8(value: u8) -> Self {
        match value & 0x0F {
            0b0111 => ExtendedFrameFormat::MulticastZoneAddressed11,
            0b0101 => ExtendedFrameFormat::MulticastZoneAddressed01,
            0b0110 => ExtendedFrameFormat::MulticastZoneAddressed10,
            0b0100 => ExtendedFrameFormat::MulticastZoneAddressed00,
            _ => ExtendedFrameFormat::PointToPointOrStandardGroupAddressed,
        }
    }
}

/// Clase para manejar el Extended Control Field (CTRLE) en un L_Data_Extended Frame TP1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtendedControlField {
    buffer: [u8; 1],
}

impl ExtendedControlField {
    /// Constructor de la clase
    /// @param input Buffer, número o array de números que representa el CTRLE (1 octeto).
    pub fn new(input: u8) -> Self {
        Self { buffer: [input] }
    }

    pub fn from_buffer(buf: &[u8]) -> Self {
        let val = if buf.is_empty() { 0 } else { buf[0] };
        Self { buffer: [val] }
    }

    /// Obtiene el buffer completo del CTRLE (1 octeto).
    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Representación en hexadecimal (2 dígitos).
    pub fn to_hex_string(&self) -> String {
        format!("{:02X}", self.buffer[0])
    }

    /// Valor numérico (0..255).
    pub fn to_number(&self) -> u8 {
        self.buffer[0]
    }

    /// Address Type (bit 7):
    ///  - 0 => Individual Address
    ///  - 1 => Group Address
    pub fn get_address_type(&self) -> AddressType {
        if (self.buffer[0] & 0x80) != 0 {
            AddressType::Group
        } else {
            AddressType::Individual
        }
    }

    pub fn set_address_type(&mut self, addr_type: AddressType) {
        match addr_type {
            AddressType::Group => self.buffer[0] |= 0x80,
            AddressType::Individual => self.buffer[0] &= 0x7F,
        }
    }

    /// Hop Count (bits 6..4) => 3 bits (0..7).
    pub fn get_hop_count(&self) -> u8 {
        (self.buffer[0] & 0x70) >> 4
    }

    pub fn set_hop_count(&mut self, value: u8) -> Result<(), &'static str> {
        if value > 7 {
            return Err("HopCount debe estar entre 0..7");
        }
        self.buffer[0] = (self.buffer[0] & 0x8F) | ((value & 0x07) << 4);
        Ok(())
    }

    /// Extended Frame Format (bits 3..0) => 4 bits (0..15).
    pub fn get_eff(&self) -> ExtendedFrameFormat {
        ExtendedFrameFormat::from_u8(self.buffer[0] & 0x0F)
    }

    pub fn set_eff(&mut self, eff: ExtendedFrameFormat) {
        self.buffer[0] = (self.buffer[0] & 0xF0) | (eff as u8 & 0x0F);
    }

    pub fn describe(&self) -> ExtendedControlFieldDescription {
        ExtendedControlFieldDescription {
            obj: "ExtendedControlField",
            hex: format!("0x{}", self.to_hex_string()),
            address_type: match self.get_address_type() {
                AddressType::Group => "GROUP(1)",
                AddressType::Individual => "INDIVIDUAL(0)",
            },
            hop_count: self.get_hop_count(),
            extended_frame_format: match self.get_eff() {
                ExtendedFrameFormat::PointToPointOrStandardGroupAddressed => {
                    "Point_To_Point_Or_Standard_Group_Addressed_L_Data_Extended_Frame"
                }
                ExtendedFrameFormat::MulticastZoneAddressed11 => "Multicast_Zone_Addressed_11",
                ExtendedFrameFormat::MulticastZoneAddressed01 => "Multicast_Zone_Addressed_01",
                ExtendedFrameFormat::MulticastZoneAddressed10 => "Multicast_Zone_Addressed_10",
                ExtendedFrameFormat::MulticastZoneAddressed00 => "Multicast_Zone_Addressed_00",
            },
            buffer: self.buffer[0],
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExtendedControlFieldDescription {
    pub obj: &'static str,
    pub hex: String,
    pub address_type: &'static str,
    pub hop_count: u8,
    pub extended_frame_format: &'static str,
    pub buffer: u8,
}
