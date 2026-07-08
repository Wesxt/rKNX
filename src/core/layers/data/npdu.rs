use crate::core::control_field_extended::AddressType;
use crate::core::layers::data::tpdu::{Tpdu, TpduDescription};
use crate::errors::KnxError;

/// Clase que representa la Network Protocol Data Unit (NPDU).
/// * Responsabilidades:
/// 1. Gestionar el Hop Count (Contador de saltos) para el enrutamiento.
/// 2. Gestionar la longitud de la trama de datos (Length).
/// 3. Encapsular el TPDU (Transport Layer PDU), que a su vez contiene el TPCI y el APDU.
/// * Estructura del Byte NPCI (Network Protocol Control Information):
/// Bits [7]   : Reservado (normalmente 0 en tramas estándar)
/// Bits [6-4] : Hop Count (0-7)
/// Bits [3-0] : Longitud del Payload (APDU)
/// @see 03_03_03 Network Layer v02.01.01 AS
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Npdu {
    hop_count: u8,
    pub tpdu: Tpdu,
    pub address_type: AddressType,
    pub length: u8,
}

impl Npdu {
    pub fn new(
        tpdu: Tpdu,
        address_type: AddressType,
        hop_count: u8,
        length: u8,
    ) -> Result<Self, KnxError> {
        if hop_count > 7 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        Ok(Self {
            hop_count,
            tpdu,
            address_type,
            length,
        })
    }

    /// Obtiene el Hop Count (0-7).
    pub fn get_hop_count(&self) -> u8 {
        self.hop_count
    }

    /// Establece el Hop Count. Retorna error si está fuera de rango (0-7).
    pub fn set_hop_count(&mut self, value: u8) -> Result<(), KnxError> {
        if value > 7 {
            return Err(KnxError::InvalidParametersForDpt);
        }
        self.hop_count = value;
        Ok(())
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let tpdu_buffer = self.tpdu.to_buffer();
        let tpdu_len = tpdu_buffer.len();

        let address_bit = match self.address_type {
            AddressType::Group => 1u8,
            AddressType::Individual => 0u8,
        };

        // Construcción del Byte NPCI (Octeto 0 del NPDU)
        // Bits 7: address_type (0)
        // Bits 6-4: Hop Count
        // Bits 3-0: Length (Longitud del TPDU)
        let npci_byte = (address_bit << 7) | (self.hop_count << 4) | (self.length & 0x0F);

        let mut buffer = vec![0u8; 1 + tpdu_len];
        buffer[0] = npci_byte;
        buffer[1..1 + tpdu_len].copy_from_slice(&tpdu_buffer);

        buffer
    }

    pub fn describe(&self) -> NpduDescription {
        NpduDescription {
            obj: "NPDU",
            layer: "Network Layer (NPDU)",
            address_type: match self.address_type {
                AddressType::Group => "GROUP",
                AddressType::Individual => "INDIVIDUAL",
            },
            hop_count: self.hop_count,
            tpdu: self.tpdu.describe(),
        }
    }

    /// Crea una instancia de NPDU a partir de un Buffer crudo.
    /// Estructura: [NPCI] [TPDU...]
    /// @param buffer El buffer completo comenzando con el byte NPCI.
    pub fn from_buffer(buffer: &[u8]) -> Result<Self, KnxError> {
        if buffer.len() < 2 {
            // Mínimo 1 byte NPCI + 1 byte payload
            return Err(KnxError::InvalidParametersForDpt);
        }

        // 1. Parsear el Byte NPCI (Octeto 0)
        // Bits 7: address_type (0)
        // Bits 6-4: Hop Count
        // Bits 3-0: Length (Longitud del TPDU)
        let npci = buffer[0];
        let hop_count = (npci >> 4) & 0x07;
        let length = npci & 0x0F;

        // Validación de longitud según especificación 03_03_03
        // Nota: 'length' en NPCI indica la longitud del TPDU en bytes.
        if buffer.len() - 1 < length as usize {
            return Err(KnxError::InvalidParametersForDpt);
        }

        // 2. Extraer el TPDU (Transport Layer PDU)
        // El payload comienza en el índice 1.
        let tpdu_buffer = &buffer[1..1 + length as usize];

        // Llamada estática recursiva a la siguiente capa
        let tpdu = Tpdu::from_buffer(tpdu_buffer)?;

        // 3. Retornar nueva instancia
        // Nota: AddressType no viene en el NPDU, viene del cEMI. Asumimos GROUP por defecto.
        Self::new(tpdu, AddressType::Group, hop_count, length)
    }
}

#[derive(Debug, Clone)]
pub struct NpduDescription {
    pub obj: &'static str,
    pub layer: &'static str,
    pub address_type: &'static str,
    pub hop_count: u8,
    pub tpdu: TpduDescription,
}
