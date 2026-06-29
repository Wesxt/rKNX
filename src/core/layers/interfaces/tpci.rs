/// Este enumerable se basa en el esquema numero 3 del titulo "2. TPDU" del documento "Transport Layer of the KNX System, Version 01.02.03"
/// @see <https://my.knx.org/es/shop/knx-specifications?product_type=knx-specifications> - "Transport Layer of the KNX System, Version 01.02.03"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tpcitype(pub u8);

impl Tpcitype {
    /// Destination address = 0
    /// - The T_Data_Broadcast service shall be applied by the user of Transport Layer, to transmit a TSDU (Transport Service Data Unit) over a connectionless communication mode to all remote partners.
    pub const T_DATA_BROADCAST_PDU: Self = Self(0x00);
    /// Destination address ≠ 0
    /// - The T_Data_Group service shall be applied by the user of Transport Layer, to transmit a TSDU (Transport Service Data Unit) over a multicast communication mode to one or more remote partners.
    pub const T_DATA_GROUP_PDU: Self = Self(0x00);
    /// The T_Data_Tag_Group-service shall be applied by the user of Transport Layer, to transmit a TSDU over a multicast communication mode to one or more remote partners.
    pub const T_DATA_TAG_GROUP_PDU: Self = Self(0x04);
    /// PDU (Protocol Data Unit) individual
    /// - The T_Data_Individual service shall be applied by the user of Transport Layer, to transmit a TSDU over a connectionless point-to-point communication mode to exactly one remote partner.
    pub const T_DATA_INDIVIDUAL_PDU: Self = Self(0x00);
    /// PDU (Protocol Data Unit) connected
    /// - The T_Data_Connected service shall b applied by the user of Transport Layer, to transmit a TSDU over a Transport Layer connection on a connection-oriented communication mode to a remote partner.
    pub const T_DATA_CONNECTED_PDU: Self = Self(0x40);
    /// Establish connection
    /// - The T_Connect service shall be applied by the user of Transport Layer, to establish a Transport Layer connection on a connection-oriented point-to-point communication mode.
    pub const T_CONNECT_PDU: Self = Self(0x80);
    /// End connection
    /// - The T_Disconnect service shall be applied by the user of Transport Layer, to release a Transport Layer connection on a connection-oriented point-to-point communication mode.
    pub const T_DISCONNECT_PDU: Self = Self(0x81);
    /// Acknowledgement
    pub const T_ACK_PDU: Self = Self(0xC2);
    /// Negative Acknowledgement
    pub const T_NAK_PDU: Self = Self(0xC3);
}

/// Clase para manejar el TPCI (Transport Protocol Control Field)
/// en KNX, correspondiente al primer octeto del TPDU (Transport Layer Protocol Data Unit).
///
/// **Estructura del TPCI (8 bits):**
///   - Bit 7         : Data/Control Flag (1 = Control, 0 = Data)
///   - Bit 6         : Numbered flag (1 = Mensaje numerado, 0 = No numerado)
///   - Bits 5..2     : Número de secuencia (4 bits: 0..15)
///   - Bits 1..0     : Reservados para el Application Layer Control Field (APCI) (se fijan a 0)
///
/// @see <https://my.knx.org/es/shop/knx-specifications?product_type=knx-specifications> - "Transport Layer of the KNX System, Version 01.02.03"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tpci {
    buffer: [u8; 1],
}

impl Tpci {
    pub fn new(initial_value: u8) -> Self {
        Self { buffer: [initial_value] }
    }

    /// Devuelve el valor crudo (8 bits) del TPCI/APCI.
    pub fn get_value(&self) -> u8 {
        self.buffer[0]
    }

    /// Asigna el valor crudo (8 bits) del TPCI.
    pub fn set_value(&mut self, value: u8) {
        self.buffer[0] = (value & 0x3F) << 2;
    }

    /// Data/Control Flag (Bit 7):
    /// - true  → Bit 7 = 1 (Control)
    /// - false → Bit 7 = 0 (Data)
    pub fn get_data_control_flag(&self) -> bool {
        ((self.buffer[0] >> 7) & 0x01) == 1
    }

    pub fn set_data_control_flag(&mut self, flag: bool) {
        if flag {
            self.buffer[0] |= 0x80;
        } else {
            self.buffer[0] &= 0x7F;
        }
    }

    /// Numbered Flag (Bit 6):
    /// - true  → Bit 6 = 1 (Mensaje numerado)
    /// - false → Bit 6 = 0 (No numerado)
    pub fn get_numbered_flag(&self) -> bool {
        ((self.buffer[0] >> 6) & 0x01) == 1
    }

    pub fn set_numbered_flag(&mut self, flag: bool) {
        if flag {
            self.buffer[0] |= 0x40;
        } else {
            self.buffer[0] &= 0xBF;
        }
    }

    /// Número de Secuencia (Bits 5 a 2):
    /// Valor de 0 a 15.
    pub fn get_sequence_number(&self) -> u8 {
        (self.buffer[0] >> 2) & 0x0F
    }

    pub fn set_sequence_number(&mut self, seq: u8) -> Result<(), &'static str> {
        if seq > 15 {
            return Err("El número de secuencia debe estar entre 0 y 15");
        }
        self.buffer[0] &= !0x3C;
        self.buffer[0] |= (seq & 0x0F) << 2;
        Ok(())
    }

    /// Bits Reservados para APCI (Bits 1 a 0).
    /// En esta clase se asumen en 0
    pub fn get_first_2bits_of_apci(&self) -> u8 {
        self.buffer[0] & 0x03
    }

    /// Primeros dos bits para el Application Layer Control Field (APCI)
    pub fn set_first_2bits_of_apci(&mut self, val: u8) -> Result<(), &'static str> {
        if val > 3 {
            return Err("Los bits reservados deben estar entre 0 y 3");
        }
        self.buffer[0] = (self.buffer[0] & 0xFC) | (val & 0x03);
        Ok(())
    }

    /// Representa el TPCI como un Buffer (1 octeto).
    pub fn to_buffer(&self) -> Vec<u8> {
        self.buffer.to_vec()
    }

    pub fn to_hex(&self) -> String {
        format!("0x{:02X}", self.buffer[0])
    }

    pub fn map_tpci_type(&self, value: u8, is_force: bool) -> &'static str {
        match value {
            0x80 => "T_CONNECT_PDU",
            0x81 => "T_DISCONNECT_PDU",
            0xC2 => "T_ACK_PDU",
            0xC3 => "T_NAK_PDU",
            _ => {
                if (value & 0xC0) == 0x40 {
                    "T_DATA_CONNECTED_PDU"
                } else if (value & 0x80) == 0x00 {
                    if (value & 0x04) == 0x04 {
                        "T_Data_Tag_Group_PDU"
                    } else {
                        "T_Data_Group_PDU"
                    }
                } else {
                    if !is_force {
                        self.map_tpci_type(value & 0xFC, true)
                    } else {
                        "Unknown PDU"
                    }
                }
            }
        }
    }

    pub fn describe(&self) -> TpciDescription {
        TpciDescription {
            obj: "TPCI",
            buffer: self.to_buffer(),
            hex: self.to_hex(),
            data_or_control_flag: if self.get_data_control_flag() { "Control" } else { "Data" },
            numbered: self.get_numbered_flag(),
            sequence_number: self.get_sequence_number(),
            first_two_bits_from_apci: self.get_first_2bits_of_apci(),
            tpci_type: self.map_tpci_type(self.get_value(), false).to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TpciDescription {
    pub obj: &'static str,
    pub buffer: Vec<u8>,
    pub hex: String,
    pub data_or_control_flag: &'static str,
    pub numbered: bool,
    pub sequence_number: u8,
    pub first_two_bits_from_apci: u8,
    pub tpci_type: String,
}
