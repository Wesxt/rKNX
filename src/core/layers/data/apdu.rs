use crate::core::layers::interfaces::apci::{Apci, ApciEnum, ApciDescription};
use crate::core::layers::interfaces::tpci::{Tpci, TpciDescription};
use crate::utils::knx_helper::KnxHelper;
use crate::errors::KnxError;

/// Clase para manejar el Application Control Field (APCI) en comunicaciones KNX,
/// específicamente para el modo T_Data_Group.
///
/// El APCI se compone de 4 o 10 bits
///
/// @see <https://my.knx.org/es/shop/knx-specifications?product_type=knx-specifications> - "Application Layer of the KNX System, Version 02.01.01"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Apdu {
    pub tpci: Tpci,
    pub apci: Apci,
    pub data: Vec<u8>,
    pub is_short: bool,
}

impl Apdu {
    pub fn new(tpci: Tpci, apci: Apci, data: Vec<u8>, is_short: bool) -> Self {
        Self {
            tpci,
            apci,
            data,
            is_short,
        }
    }

    pub fn get_length(&self) -> usize {
        KnxHelper::get_data_length(&self.data, self.is_short)
    }

    /// Devuelve un buffer con TPCI/APCI + data
    pub fn to_buffer(&self) -> Vec<u8> {
        let mut tpci_copy = Tpci::new(self.tpci.get_value());
        if tpci_copy.get_data_control_flag() {
            return vec![tpci_copy.get_value()];
        }

        let len = self.get_length();
        let mut buffer = vec![0u8; 1 + len];
        let pack_number = self.apci.pack_number();
        let _ = tpci_copy.set_first_2bits_of_apci(pack_number.0);

        // TPCI/APCI
        buffer[0] = tpci_copy.get_value();
        if buffer.len() > 1 {
            buffer[1] = pack_number.1;
        }

        // Data
        KnxHelper::write_data(&mut buffer, &self.data, 1, self.is_short);
        buffer
    }

    /// Crea una instancia de APDU reconstruyendo APCI y Datos.
    /// Maneja la lógica de bits mezclados según Spec 03.03.07.
    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.is_empty() {
            return Err(KnxError::InvalidParametersForDpt);
        }

        // 1. Reconstruir el APCI (10 bits)
        // Byte 0: [T T T T T T A9 A8]  -> Nos interesan los últimos 2 bits
        // Byte 1 puede contener los 8 bits bajos del APCI en servicios extendidos
        // o solo los 4 bits altos del APCI en servicios optimizados con datos cortos.

        let byte0 = buffer[0];
        let byte1 = if buffer.len() > 1 { buffer[1] } else { 0 };

        // ** Se evita usar la mascara 0xfc para que el tpci tenga los dos bits más
        // ** significativos del APCI
        let tpci = Tpci::new(byte0);

        // Parte Alta APCI (2 bits): Byte 0 & 0000 0011
        let apci_high = (byte0 & 0x03) as u16;

        let full_apci_value = (apci_high << 8) | byte1 as u16;
        let short_apci_value = (apci_high << 8) | ((byte1 & 0xC0) as u16);
        let has_full_apci_command = ApciEnum::from_u16(full_apci_value).is_some();

        let apci_value = if buffer.len() > 2 || has_full_apci_command {
            full_apci_value
        } else {
            short_apci_value
        };

        let apci = Apci::new(apci_value);

        // 2. Extraer los Datos (Payload)
        // Regla de longitud KNX:
        // Si el TPDU tiene longitud > 2 bytes, los datos comienzan en el byte 2 (Extended Data).
        // Si el TPDU tiene longitud == 2 bytes, los datos son los 6 bits bajos del byte 1 (Optimized/Short Data).

        let (data, is_short) = if buffer.len() > 2 {
            // Caso: Datos largos (> 6 bits o estructurados)
            // Ejemplo: Escribir un flotante (4 bytes) -> buffer total 1 + 1 + 4 = 6 bytes.
            // Los datos empiezan en el índice 2.
            (buffer[2..].to_vec(), false)
        } else if has_full_apci_command && full_apci_value != short_apci_value {
            (Vec::new(), false)
        } else {
            // Caso: Datos cortos (<= 6 bits)
            // Ejemplo: Escribir Booleano (ON/OFF) o 3-bit scaling.
            // Los datos están en los últimos 6 bits del byte 1.
            // Máscara: 0011 1111 (0x3F)
            if buffer.len() == 2 {
                let short_data = byte1 & 0x3F;
                // Lo convertimos a Buffer de 1 byte para mantener consistencia
                (vec![short_data], true)
            } else {
                // Caso raro: Longitud 1 (Solo comando sin datos, ej. Read request)
                (Vec::new(), false)
            }
        };

        Ok(Self::new(tpci, apci, data, is_short))
    }

    pub fn describe(&self) -> ApduDescription {
        ApduDescription {
            obj: "APDU",
            layer: "Application Layer (APDU)",
            tpci: self.tpci.describe(),
            apci: self.apci.describe(),
            data: self.data.clone(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ApduDescription {
    pub obj: &'static str,
    pub layer: &'static str,
    pub tpci: TpciDescription,
    pub apci: ApciDescription,
    pub data: Vec<u8>,
}
