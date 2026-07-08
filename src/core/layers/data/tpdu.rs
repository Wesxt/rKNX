use crate::core::layers::data::apdu::{Apdu, ApduDescription};
use crate::core::layers::interfaces::tpci::{Tpci, TpciDescription};
use crate::errors::KnxError;
use crate::utils::knx_helper::KnxHelper;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tpdu {
    pub tpci: Tpci,
    pub apdu: Apdu,
    pub data: Vec<u8>,
}

impl Tpdu {
    pub fn new(tpci: Tpci, apdu: Apdu, data: Vec<u8>) -> Self {
        Self { tpci, apdu, data }
    }

    /// Get length all TPDU
    pub fn get_length(&self) -> usize {
        self.to_buffer().len()
    }

    /// Devuelve un buffer con TPCI/APCI + data
    pub fn to_buffer(&self) -> Vec<u8> {
        let mut tpci_copy = Tpci::new(self.tpci.get_value());
        if tpci_copy.get_data_control_flag() {
            return vec![tpci_copy.get_value()];
        }

        let len = self.apdu.get_length();
        let mut buffer = vec![0u8; 1 + len];
        let pack_number = self.apdu.apci.pack_number();
        let _ = tpci_copy.set_first_2bits_of_apci(pack_number.0);

        buffer[1] = pack_number.1;
        buffer[0] = tpci_copy.get_value();

        KnxHelper::write_data(&mut buffer, &self.data, 1, self.apdu.is_short);
        buffer
    }

    pub fn describe(&self) -> TpduDescription {
        TpduDescription {
            obj: "TPDU",
            layer: "Transport Layer (TPDU)",
            tpci: self.tpci.describe(),
            apdu: self.apdu.describe(),
        }
    }

    /// Crea una instancia de TPDU.
    /// Estructura: [TPCI + APCI_High] [APCI_Low + Data] [Data...]
    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.is_empty() {
            return Err(KnxError::InvalidParametersForDpt);
        }

        // 1. Extraer TPCI (Transport Protocol Control Information)
        // El TPCI ocupa los primeros 6 bits del primer octeto.
        // Máscara: 1111 1100 (0xFC)
        // ** Se evita usar la mascara 0xfc para que el tpci tenga los dos bits más
        // ** significativos del APCI
        let tpci_byte = buffer[0];
        let tpci = Tpci::new(tpci_byte);

        // 2. Extraer APDU (Application Protocol Data Unit)
        // IMPORTANTE: Pasamos TODO el buffer, porque el APDU necesita los
        // últimos 2 bits del primer byte (que son parte del APCI).
        let apdu = Apdu::from_buffer(buffer)?;
        let data = apdu.data.clone();

        Ok(Self::new(tpci, apdu, data))
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TpduDescription {
    pub obj: &'static str,
    pub layer: &'static str,
    pub tpci: TpciDescription,
    pub apdu: ApduDescription,
}
