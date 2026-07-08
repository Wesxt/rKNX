#![allow(async_fn_in_trait)]

pub mod router;
pub mod server;
pub mod tpuart;
pub mod tunnel_connection;
pub mod tunneling;
pub mod usb;

use crate::core::cemi::{Cemi, LData};
use crate::core::control_field::ControlField;
use crate::core::control_field_extended::ExtendedControlField;
use crate::core::data::knx_data_decode::DptValue;
use crate::core::data::knx_data_encode::KnxDataEncoder;
use crate::core::layers::data::apdu::Apdu;
use crate::core::layers::data::tpdu::Tpdu;
use crate::core::layers::interfaces::apci::{Apci, ApciEnum};
use crate::core::layers::interfaces::tpci::Tpci;
use crate::errors::KnxError;

pub trait KnxService: Send + Sync {
    async fn connect(&self) -> Result<(), KnxError>;
    async fn disconnect(&self) -> Result<(), KnxError>;
    async fn send(&self, cemi: &Cemi) -> Result<(), KnxError>;
    fn connection_state(&self) -> String;
    fn is_connected(&self) -> bool;
    fn individual_address(&self) -> String;

    async fn write(&self, destination: &str, dpt: &str, value: &DptValue) -> Result<(), KnxError> {
        let data = KnxDataEncoder::encode_this(dpt, value)?;
        let is_short = KnxDataEncoder::is_short_dpt(dpt);

        let cf1 = ControlField::new(0xBC);
        let cf2 = ExtendedControlField::new(0xE0);

        let tpci = Tpci::new(0x00);
        let apci = Apci::new(ApciEnum::AGroupValueWrite as u16);
        let tpdu = Tpdu {
            tpci: tpci.clone(),
            apdu: Apdu {
                tpci: tpci.clone(),
                apci,
                data: data.clone(),
                is_short,
            },
            data,
        };

        let cemi = Cemi::LDataReq(LData {
            additional_info: Vec::new(),
            control_field1: cf1,
            control_field2: cf2,
            source_address: self.individual_address(),
            destination_address: destination.to_string(),
            tpdu,
        });

        self.send(&cemi).await
    }

    async fn read(&self, destination: &str) -> Result<(), KnxError> {
        let cf1 = ControlField::new(0xBC);
        let cf2 = ExtendedControlField::new(0xE0);

        let tpci = Tpci::new(0x00);
        let apci = Apci::new(ApciEnum::AGroupValueRead as u16);
        let tpdu = Tpdu {
            tpci: tpci.clone(),
            apdu: Apdu {
                tpci: tpci.clone(),
                apci,
                data: vec![0],
                is_short: true,
            },
            data: vec![0],
        };

        let cemi = Cemi::LDataReq(LData {
            additional_info: Vec::new(),
            control_field1: cf1,
            control_field2: cf2,
            source_address: "0.0.0".to_string(),
            destination_address: destination.to_string(),
            tpdu,
        });

        self.send(&cemi).await
    }
}
