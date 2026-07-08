use crate::core::cemi::{Cemi, LBusmon, LData, LRaw};
use crate::core::control_field::ControlField;
use crate::core::control_field_extended::ExtendedControlField;
use crate::core::emi::{Emi, LBusmonEmi, LDataEmi, LRawEmi};
use crate::core::layers::data::npdu::Npdu;
use crate::errors::KnxError;

pub struct CemiAdapter;

impl CemiAdapter {
    pub fn emi_to_cemi(emi_buffer: &[u8]) -> Result<Cemi, KnxError> {
        let emi = Emi::from_buffer(emi_buffer)?;
        match emi {
            Emi::LDataReq(ld) | Emi::LDataCon(ld) | Emi::LDataInd(ld) => {
                let mut control_field2 = ExtendedControlField::new(0);
                let _ = control_field2.set_hop_count(ld.npdu.get_hop_count());
                control_field2.set_address_type(ld.npdu.address_type);

                let cemi_data = LData {
                    additional_info: Vec::new(),
                    control_field1: ld.control_field1,
                    control_field2,
                    source_address: ld.source_address,
                    destination_address: ld.destination_address,
                    tpdu: ld.npdu.tpdu,
                };

                let req_mc = crate::core::message_code_field::get_emi_message_code("L_Data.req")
                    .unwrap_or(0x11);
                let con_mc = crate::core::message_code_field::get_emi_message_code("L_Data.con")
                    .unwrap_or(0x2E);

                if ld.message_code == req_mc {
                    Ok(Cemi::LDataReq(cemi_data))
                } else if ld.message_code == con_mc {
                    Ok(Cemi::LDataCon(cemi_data))
                } else {
                    Ok(Cemi::LDataInd(cemi_data))
                }
            }
            Emi::LBusmonInd(lb) => Ok(Cemi::LBusmonInd(LBusmon {
                additional_info: Vec::new(),
                data: lb.data,
            })),
            Emi::LRawReq(lr) | Emi::LRawCon(lr) | Emi::LRawInd(lr) => {
                let cemi_raw = LRaw {
                    additional_info: Vec::new(),
                    data: lr.data,
                };
                let req_mc = crate::core::message_code_field::get_emi_message_code("L_Raw.req")
                    .unwrap_or(0x10);
                let con_mc = crate::core::message_code_field::get_emi_message_code("L_Raw.con")
                    .unwrap_or(0x2F);

                if lr.message_code == req_mc {
                    Ok(Cemi::LRawReq(cemi_raw))
                } else if lr.message_code == con_mc {
                    Ok(Cemi::LRawCon(cemi_raw))
                } else {
                    Ok(Cemi::LRawInd(cemi_raw))
                }
            }
            _ => Err(KnxError::InvalidParametersForDpt),
        }
    }

    pub fn cemi_to_emi(cemi: &Cemi) -> Result<Emi, KnxError> {
        match cemi {
            Cemi::LDataReq(ld) | Cemi::LDataCon(ld) | Cemi::LDataInd(ld) => {
                let tpdu_buffer = ld.tpdu.to_buffer();
                let npdu = Npdu::new(
                    ld.tpdu.clone(),
                    ld.control_field2.get_address_type(),
                    ld.control_field2.get_hop_count(),
                    tpdu_buffer.len() as u8,
                )?;

                let ld_emi = LDataEmi {
                    message_code: cemi.get_message_code(),
                    control_field1: ControlField::new(ld.control_field1.get_buffer()[0]),
                    source_address: ld.source_address.clone(),
                    destination_address: ld.destination_address.clone(),
                    npdu,
                };

                let req_mc = crate::core::message_code_field::get_cemi_message_code("L_Data.req")
                    .unwrap_or(0x11);
                let con_mc = crate::core::message_code_field::get_cemi_message_code("L_Data.con")
                    .unwrap_or(0x2E);
                let cemi_mc = cemi.get_message_code();

                if cemi_mc == req_mc {
                    Ok(Emi::LDataReq(ld_emi))
                } else if cemi_mc == con_mc {
                    Ok(Emi::LDataCon(ld_emi))
                } else {
                    Ok(Emi::LDataInd(ld_emi))
                }
            }
            Cemi::LBusmonInd(lb) => {
                let busmon_mc =
                    crate::core::message_code_field::get_emi_message_code("L_Busmon.ind")
                        .unwrap_or(0x2B);
                Ok(Emi::LBusmonInd(LBusmonEmi {
                    message_code: busmon_mc,
                    data: lb.data.clone(),
                }))
            }
            Cemi::LRawReq(lr) | Cemi::LRawCon(lr) | Cemi::LRawInd(lr) => {
                let lr_emi = LRawEmi {
                    message_code: cemi.get_message_code(),
                    data: lr.data.clone(),
                };
                let req_mc = crate::core::message_code_field::get_cemi_message_code("L_Raw.req")
                    .unwrap_or(0x10);
                let con_mc = crate::core::message_code_field::get_cemi_message_code("L_Raw.con")
                    .unwrap_or(0x2F);
                let cemi_mc = cemi.get_message_code();

                if cemi_mc == req_mc {
                    Ok(Emi::LRawReq(lr_emi))
                } else if cemi_mc == con_mc {
                    Ok(Emi::LRawCon(lr_emi))
                } else {
                    Ok(Emi::LRawInd(lr_emi))
                }
            }
            _ => Err(KnxError::InvalidParametersForDpt),
        }
    }
}
