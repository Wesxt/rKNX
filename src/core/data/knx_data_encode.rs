use crate::errors::KnxError;
use crate::core::data::knx_data_decode::{DptValue, KnxDataDecode};
use byteorder::{BigEndian, ByteOrder};

pub struct KnxDataEncoder;

impl KnxDataEncoder {
    pub fn get_dpt_number(dpt: &str) -> Option<u32> {
        KnxDataDecode::get_dpt_number(dpt)
    }

    pub fn fallback_dpt(dpt_num: u32) -> u32 {
        KnxDataDecode::fallback_dpt(dpt_num)
    }

    pub fn is_short_dpt(dpt: &str) -> bool {
        if let Some(dpt_num) = Self::get_dpt_number(dpt) {
            let main = dpt_num / 1000;
            let m = if main == 0 { dpt_num } else { main };
            return m >= 1 && m <= 3;
        }
        false
    }

    pub fn encode_this(dpt: &str, data: &DptValue) -> Result<Vec<u8>, KnxError> {
        let dpt_num = Self::get_dpt_number(dpt).ok_or(KnxError::DPTNotFound)?;
        let resolved = Self::fallback_dpt(dpt_num);

        match (resolved, data) {
            (1, DptValue::Dpt1(b)) => {
                Ok(vec![if *b { 0x01 } else { 0x00 }])
            }
            (2, DptValue::Dpt2(v)) => {
                Ok(vec![(v.control << 1) | (v.value & 0x01)])
            }
            (3007 | 3008, DptValue::Dpt3(v)) => {
                Ok(vec![(v.control << 3) | (v.step_code & 0x07)])
            }
            (4001 | 4002, DptValue::Dpt4(c)) => {
                let val = *c as u8;
                if resolved == 4001 && (val & 0x80) != 0 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                Ok(vec![val])
            }
            (5, DptValue::Dpt5(v)) => {
                Ok(vec![*v])
            }
            (5001, DptValue::Dpt5001(s)) => {
                // Parse percent string like "50.0%"
                let s_clean = s.trim_end_matches('%');
                let pct = s_clean.parse::<f32>().map_err(|_| KnxError::InvalidParametersForDpt)?;
                let byte_val = ((pct / 100.0) * 255.0).round() as u8;
                Ok(vec![byte_val])
            }
            (5002, DptValue::Dpt5002(s)) => {
                // Parse angle string like "180.0ª"
                let s_clean = s.trim_end_matches('ª').trim_end_matches('°');
                let angle = s_clean.parse::<f32>().map_err(|_| KnxError::InvalidParametersForDpt)?;
                let byte_val = ((angle / 360.0) * 255.0).round() as u8;
                Ok(vec![byte_val])
            }
            (6, DptValue::Dpt6(v)) => {
                Ok(vec![*v as u8])
            }
            (6001, DptValue::Dpt6001(s)) => {
                let s_clean = s.trim_end_matches('%');
                let pct = s_clean.parse::<i8>().map_err(|_| KnxError::InvalidParametersForDpt)?;
                Ok(vec![pct as u8])
            }
            (6010, DptValue::Dpt6010(s)) => {
                let s_clean = s.trim_end_matches(" counter pulses").trim();
                let pulses = s_clean.parse::<i8>().map_err(|_| KnxError::InvalidParametersForDpt)?;
                Ok(vec![pulses as u8])
            }
            (6020, DptValue::Dpt6020(v)) => {
                let status = if v.status == "Activo" { 1u8 } else { 0u8 };
                let mode = match v.mode.as_str() {
                    "Modo 0 activo" => 0b001u8,
                    "Modo 1 activo" => 0b010u8,
                    "Modo 2 activo" => 0b100u8,
                    _ => 0,
                };
                Ok(vec![(status << 3) | mode])
            }
            (7, DptValue::Dpt7(v)) => {
                let mut buf = vec![0u8; 2];
                BigEndian::write_u16(&mut buf, *v);
                Ok(buf)
            }
            (7001 | 7002 | 7003 | 7004 | 7005 | 7006 | 7007 | 7011 | 7013, DptValue::Dpt7Suffix(s)) => {
                // Determine multipliers based on resolved sub-types
                let val = match resolved {
                    7001 => {
                        let parsed = s.trim_end_matches("pulses").trim().parse::<f32>()
                            .map_err(|_| KnxError::InvalidParametersForDpt)?;
                        parsed as u16
                    }
                    7002 => {
                        let parsed = s.trim_end_matches("ms").trim().parse::<f32>()
                            .map_err(|_| KnxError::InvalidParametersForDpt)?;
                        parsed as u16
                    }
                    7003 => {
                        let parsed = s.trim_end_matches('s').trim().parse::<f32>()
                            .map_err(|_| KnxError::InvalidParametersForDpt)?;
                        (parsed * 100.0).round() as u16
                    }
                    7004 => {
                        let parsed = s.trim_end_matches('s').trim().parse::<f32>()
                            .map_err(|_| KnxError::InvalidParametersForDpt)?;
                        (parsed * 10.0).round() as u16
                    }
                    7005 => {
                        let parsed = s.trim_end_matches('s').trim().parse::<f32>()
                            .map_err(|_| KnxError::InvalidParametersForDpt)?;
                        parsed as u16
                    }
                    7006 => {
                        let parsed = s.trim_end_matches("min").trim().parse::<f32>()
                            .map_err(|_| KnxError::InvalidParametersForDpt)?;
                        parsed as u16
                    }
                    7007 => {
                        let parsed = s.trim_end_matches('h').trim().parse::<f32>()
                            .map_err(|_| KnxError::InvalidParametersForDpt)?;
                        parsed as u16
                    }
                    7011 => {
                        let parsed = s.trim_end_matches("mm").trim().parse::<f32>()
                            .map_err(|_| KnxError::InvalidParametersForDpt)?;
                        parsed as u16
                    }
                    7013 => {
                        let parsed = s.trim_end_matches("lux").trim().parse::<f32>()
                            .map_err(|_| KnxError::InvalidParametersForDpt)?;
                        parsed as u16
                    }
                    _ => 0,
                };
                let mut buf = vec![0u8; 2];
                BigEndian::write_u16(&mut buf, val);
                Ok(buf)
            }
            (7012, DptValue::Dpt7012(v)) => {
                let parsed = v.value.trim_end_matches("mA").trim().parse::<u16>()
                    .unwrap_or(0);
                let mut buf = vec![0u8; 2];
                BigEndian::write_u16(&mut buf, parsed);
                Ok(buf)
            }
            (8, DptValue::Dpt8(v)) => {
                let mut buf = vec![0u8; 2];
                BigEndian::write_i16(&mut buf, *v);
                Ok(buf)
            }
            (9, DptValue::Dpt9(f)) => {
                let val = *f;
                if val.is_nan() || val.is_infinite() {
                    return Ok(vec![0x7F, 0xFF]);
                }
                let mut m = val / 0.01;
                let mut e = 0;
                while (m > 2047.0 || m < -2048.0) && e < 15 {
                    m /= 2.0;
                    e += 1;
                }
                let mut m_int = m.round() as i32;
                if e == 15 && m_int > 2046 {
                    m_int = 2046;
                }
                let m_encoded = (m_int & 0x07FF) as u16;
                let sign = if m_int < 0 { 1u16 } else { 0u16 };
                let encoded = (sign << 15) | ((e as u16) << 11) | m_encoded;
                let mut buf = vec![0u8; 2];
                BigEndian::write_u16(&mut buf, encoded);
                Ok(buf)
            }
            (10001, DptValue::Dpt10(v)) => {
                let mut buf = vec![0u8; 3];
                buf[0] = ((v.day & 0x07) << 5) | (v.hour & 0x1F);
                buf[1] = v.minutes & 0x3F;
                buf[2] = v.seconds & 0x3F;
                Ok(buf)
            }
            (11001, DptValue::Dpt11(v)) => {
                let mut buf = vec![0u8; 3];
                buf[0] = v.day & 0x1F;
                buf[1] = v.month & 0x0F;
                let yr = if v.year >= 2000 {
                    (v.year - 2000) as u8
                } else if v.year >= 1900 {
                    (v.year - 1900) as u8
                } else {
                    v.year as u8
                };
                buf[2] = yr & 0x7F;
                Ok(buf)
            }
            (12001 | 12002, DptValue::Dpt12(v)) => {
                let mut buf = vec![0u8; 4];
                BigEndian::write_u32(&mut buf, *v);
                Ok(buf)
            }
            (13 | 13001 | 13002 | 13010 | 13011 | 13012 | 13013 | 13014 | 13015 | 13016 | 13100, DptValue::Dpt13(v)) => {
                let mut buf = vec![0u8; 4];
                BigEndian::write_i32(&mut buf, *v);
                Ok(buf)
            }
            (14, DptValue::Dpt14(f)) => {
                let mut buf = vec![0u8; 4];
                BigEndian::write_f32(&mut buf, *f);
                Ok(buf)
            }
            (16 | 16002 | 28001, DptValue::Dpt16(s)) => {
                let mut buf = vec![0u8; 14];
                let bytes = s.as_bytes();
                let limit = bytes.len().min(14);
                buf[..limit].copy_from_slice(&bytes[..limit]);
                Ok(buf)
            }
            (20 | 20001 | 20002 | 20003 | 20004 | 20005 | 20006 | 20007 | 20008 | 20011 | 20012 | 20013 | 20014 | 20017 | 20020 | 20021 | 20022, DptValue::Dpt20(v)) => {
                Ok(vec![*v])
            }
            (232600, DptValue::Dpt232(v)) => {
                Ok(vec![v.r, v.g, v.b])
            }
            (251600, DptValue::Dpt251(v)) => {
                let mut buf = vec![0u8; 6];
                buf[0] = v.r.value;
                buf[1] = v.g.value;
                buf[2] = v.b.value;
                buf[3] = v.w.value;
                buf[4] = 0; // Reserved
                
                let mut validity_bits = 0u8;
                if v.r.valid { validity_bits |= 0x08; }
                if v.g.valid { validity_bits |= 0x04; }
                if v.b.valid { validity_bits |= 0x02; }
                if v.w.valid { validity_bits |= 0x01; }
                buf[5] = validity_bits;
                Ok(buf)
            }
            (_, DptValue::Raw(v)) => {
                Ok(v.clone())
            }
            _ => Err(KnxError::InvalidParametersForDpt),
        }
    }
}
