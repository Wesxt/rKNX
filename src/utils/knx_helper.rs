//           +-----------------------------------------------+
// 16 bits   |              INDIVIDUAL ADDRESS               |
//           +-----------------------+-----------------------+
//           | OCTET 0 (high byte)   |  OCTET 1 (low byte)   |
//           +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//    bits   | 7| 6| 5| 4| 3| 2| 1| 0| 7| 6| 5| 4| 3| 2| 1| 0|
//           +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//           |  Subnetwork Address   |                       |
//           +-----------+-----------+     Device Address    |
//           |(Area Adrs)|(Line Adrs)|                       |
//           +-----------------------+-----------------------+

//           +-----------------------------------------------+
// 16 bits   |             GROUP ADDRESS (3 level)           |
//           +-----------------------+-----------------------+
//           | OCTET 0 (high byte)   |  OCTET 1 (low byte)   |
//           +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//    bits   | 7| 6| 5| 4| 3| 2| 1| 0| 7| 6| 5| 4| 3| 2| 1| 0|
//           +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//           |  | Main Grp  | Midd G |       Sub Group       |
//           +--+--------------------+-----------------------+

//           +-----------------------------------------------+
// 16 bits   |             GROUP ADDRESS (2 level)           |
//           +-----------------------+-----------------------+
//           | OCTET 0 (high byte)   |  OCTET 1 (low byte)   |
//           +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//    bits   | 7| 6| 5| 4| 3| 2| 1| 0| 7| 6| 5| 4| 3| 2| 1| 0|
//           +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//           |  | Main Grp  |            Sub Group           |
//           +--+--------------------+-----------------------+

use crate::errors::KnxError;
use byteorder::{BigEndian, ByteOrder};

/// This helper is completely based on the knx.js repo and KNX.ts.
/// Contains utility functions for encoding, decoding, and validating addresses.
pub struct KnxHelper;

impl KnxHelper {
    /// Converts a KNX address (string) into a 2-byte array.
    /// This method maps to the `GetAddress_` method.
    pub fn get_address_from_string(address: &str) -> Result<[u8; 2], KnxError> {
        let mut addr = [0u8; 2];
        let mut three_level_addressing = true;
        let group = address.contains('/');
        
        let parts: Vec<&str> = if !group {
            address.split('.').collect()
        } else {
            address.split('/').collect()
        };

        if !group {
            if parts.len() != 3 || parts[0].len() > 2 || parts[1].len() > 2 || parts[2].len() > 3 {
                return Err(KnxError::InvalidKnxAddressException(address.to_string()));
            }
        } else {
            if parts.len() != 3 || parts[0].len() > 2 || parts[1].len() > 1 || parts[2].len() > 3 {
                if parts.len() != 2 || parts[0].len() > 2 || parts[1].len() > 4 {
                    return Err(KnxError::InvalidKnxAddressException(address.to_string()));
                }
                three_level_addressing = false;
            }
        }

        if !three_level_addressing {
            let mut part = parts[0].parse::<u32>().map_err(|_| KnxError::InvalidKnxAddressException(address.to_string()))?;
            if part > 15 {
                return Err(KnxError::InvalidKnxAddressException(address.to_string()));
            }
            addr[0] = ((part << 3) & 255) as u8;
            part = parts[1].parse::<u32>().map_err(|_| KnxError::InvalidKnxAddressException(address.to_string()))?;
            if part > 2047 {
                return Err(KnxError::InvalidKnxAddressException(address.to_string()));
            }
            let mut part2 = [0u8; 2];
            BigEndian::write_u16(&mut part2, part as u16);
            addr[0] = ((addr[0] as u32 | part2[0] as u32) & 255) as u8;
            addr[1] = part2[1];
        } else {
            let mut part = parts[0].parse::<u32>().map_err(|_| KnxError::InvalidKnxAddressException(address.to_string()))?;
            if part > 31 {
                return Err(KnxError::InvalidKnxAddressException(address.to_string()));
            }
            addr[0] = (if group { (part << 3) & 255 } else { (part << 4) & 255 }) as u8;
            part = parts[1].parse::<u32>().map_err(|_| KnxError::InvalidKnxAddressException(address.to_string()))?;
            if (group && part > 7) || (!group && part > 15) {
                return Err(KnxError::InvalidKnxAddressException(address.to_string()));
            }
            addr[0] = ((addr[0] as u32 | part) & 255) as u8;
            part = parts[2].parse::<u32>().map_err(|_| KnxError::InvalidKnxAddressException(address.to_string()))?;
            if part > 255 {
                return Err(KnxError::InvalidKnxAddressException(address.to_string()));
            }
            addr[1] = (part & 255) as u8;
        }

        Ok(addr)
    }

    /// Converts a raw KNX address buffer (slice) to its string representation.
    pub fn get_address_to_string(
        addr: &[u8],
        separator: &str,
        three_level_addressing: bool,
    ) -> Result<String, KnxError> {
        if addr.len() < 2 {
            return Err(KnxError::InvalidKnxAddressException(
                "Address buffer must be at least 2 bytes".to_string(),
            ));
        }

        let high_byte = addr[0];
        let low_byte = addr[1];
        let is_group = separator == "/";

        if is_group && !three_level_addressing {
            let main = high_byte >> 3;
            let sub = (((high_byte & 0x07) as u16) << 8) | low_byte as u16;
            return Ok(format!("{}/{}", main, sub));
        }

        if is_group {
            let main = (high_byte >> 3) & 0x1f;
            let middle = high_byte & 0x07;
            let sub = low_byte;
            return Ok(format!("{}/{}/{}", main, middle, sub));
        }

        let area = high_byte >> 4;
        let line = high_byte & 0x0f;
        let device = low_byte;
        Ok(format!("{}.{}.{}", area, line, device))
    }

    /// Converts an address number to its string representation.
    pub fn get_address_from_number(
        addr: u16,
        separator: &str,
        three_level_addressing: bool,
    ) -> Result<String, KnxError> {
        let mut buffer = [0u8; 2];
        BigEndian::write_u16(&mut buffer, addr);
        Self::get_address_to_string(&buffer, separator, three_level_addressing)
    }

    /// Converts a KNX address (string) into a 2-byte array with custom parameters.
    /// This method maps to the `addressToBuffer` method.
    pub fn address_to_buffer(
        address: &str,
        separator: &str,
        group: bool,
        three_level_addressing: bool,
    ) -> Result<[u8; 2], KnxError> {
        let parts: Vec<&str> = address.split(separator).collect();
        let min_len = if three_level_addressing { 3 } else { 2 };
        if parts.len() < min_len {
            return Err(KnxError::InvalidKnxAddressException(
                "Invalid address. Incorrect format.".to_string(),
            ));
        }

        let mut parts_num = Vec::new();
        for p in parts {
            match p.parse::<u32>() {
                Ok(n) => parts_num.push(n),
                Err(_) => {
                    return Err(KnxError::InvalidKnxAddressException(
                        "Invalid address. Incorrect format.".to_string(),
                    ))
                }
            }
        }

        let mut addr = [0u8; 2];

        if group {
            if three_level_addressing {
                if parts_num[0] > 31 || parts_num[1] > 7 || parts_num[2] > 255 {
                    return Err(KnxError::InvalidKnxAddressException(
                        "Invalid group address (3 levels)".to_string(),
                    ));
                }
                addr[0] = (((parts_num[0] & 0x1f) << 3) | (parts_num[1] & 0x07)) as u8;
                addr[1] = (parts_num[2] & 0xff) as u8;
            } else {
                if parts_num[0] > 31 || parts_num[1] > 2047 {
                    return Err(KnxError::InvalidKnxAddressException(
                        "Invalid group address (2 levels)".to_string(),
                    ));
                }
                addr[0] = (((parts_num[0] & 0x1f) << 3) | ((parts_num[1] >> 8) & 0x07)) as u8;
                addr[1] = (parts_num[1] & 0xff) as u8;
            }
        } else {
            if parts_num[0] > 15 || parts_num[1] > 15 || parts_num[2] > 255 {
                return Err(KnxError::InvalidKnxAddressException(
                    "Invalid individual address.".to_string(),
                ));
            }
            addr[0] = (((parts_num[0] & 0x0f) << 4) | (parts_num[1] & 0x0f)) as u8;
            addr[1] = (parts_num[2] & 0xff) as u8;
        }

        Ok(addr)
    }

    /// Obtains the data length.
    pub fn get_data_length(data: &[u8], is_short: bool) -> usize {
        if data.is_empty() {
            return 0;
        }
        if is_short {
            return 1;
        }
        data.len() + 1
    }

    /// Escribe los datos en el datagrama.
    /// @param datagram PDU
    /// @param data
    /// @param data_start Debe ser siempre en el indice donde están los bits menos significativos del APCI
    /// @param is_short Si es true, el dato se incrusta en los 6 bits del APCI (DPT 1, 2, 3)
    pub fn write_data(datagram: &mut [u8], data: &[u8], data_start: usize, is_short: bool) {
        if data.is_empty() {
            return;
        }

        // ESTRATEGIA: Optimización "Short Data" (6 bits)
        // Si es 1 byte y el valor es pequeño (<= 0x3F), asumimos que es DPT1/2/3 y lo incrustamos.
        if is_short {
            datagram[data_start] = (datagram[data_start] & 0xc0) | (data[0] & 0x3f);
            return;
        }

        // ESTRATEGIA: Datos estándar (> 6 bits o arrays largos)
        // Se escriben a partir del siguiente byte (data_start + 1)
        for (i, &byte) in data.iter().enumerate() {
            datagram[data_start + 1 + i] = byte;
        }
    }

    /// Verifica si una dirección de grupo KNX es válida.
    pub fn is_valid_group_address(address: &str) -> bool {
        let parts3: Vec<&str> = address.split('/').collect();
        if parts3.len() == 3 {
            let main = parts3[0].parse::<u32>();
            let middle = parts3[1].parse::<u32>();
            let sub = parts3[2].parse::<u32>();
            if let (Ok(main), Ok(middle), Ok(sub)) = (main, middle, sub) {
                return main <= 31 && middle <= 7 && sub <= 255;
            }
        } else if parts3.len() == 2 {
            let main = parts3[0].parse::<u32>();
            let sub = parts3[1].parse::<u32>();
            if let (Ok(main), Ok(sub)) = (main, sub) {
                return main <= 31 && sub <= 2047;
            }
        } else if parts3.len() == 1 {
            if let Ok(value) = address.parse::<u32>() {
                return value <= 65535;
            }
        }
        false
    }

    /// Verifica si una dirección individual KNX es válida.
    pub fn is_valid_individual_address(address: &str) -> bool {
        let parts: Vec<&str> = address.split('.').collect();
        if parts.len() != 3 {
            return false;
        }
        let area = parts[0].parse::<u32>();
        let line = parts[1].parse::<u32>();
        let device = parts[2].parse::<u32>();
        if let (Ok(area), Ok(line), Ok(device)) = (area, line, device) {
            return area <= 15 && line <= 15 && device <= 255;
        }
        false
    }

    /// Verifica si un Buffer de 2 octetos representa una dirección de grupo KNX válida.
    pub fn is_valid_group_address_buffer(buffer: &[u8]) -> bool {
        buffer.len() == 2
    }
}

