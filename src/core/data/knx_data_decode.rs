use crate::errors::KnxError;
use byteorder::{BigEndian, ByteOrder};

#[derive(Debug, Clone, PartialEq)]
pub struct Dpt2Value {
    pub control: u8,
    pub value: u8,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpt3Value {
    pub control: u8,
    pub step_code: u8,
    pub action: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpt6020Value {
    pub status: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpt7012Value {
    pub value: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpt10Value {
    pub day: u8,
    pub day_name: String,
    pub hour: u8,
    pub minutes: u8,
    pub seconds: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpt11Value {
    pub day: u8,
    pub month: u8,
    pub year: u16,
    pub formatted: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpt232Value {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpt251Val {
    pub value: u8,
    pub valid: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpt251Value {
    pub r: Dpt251Val,
    pub g: Dpt251Val,
    pub b: Dpt251Val,
    pub w: Dpt251Val,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DptValue {
    Dpt1(bool),
    Dpt2(Dpt2Value),
    Dpt3(Dpt3Value),
    Dpt4(char),
    Dpt5(u8),
    Dpt5001(String),
    Dpt5002(String),
    Dpt6(i8),
    Dpt6001(String),
    Dpt6010(String),
    Dpt6020(Dpt6020Value),
    Dpt7(u16),
    Dpt7Suffix(String),
    Dpt7012(Dpt7012Value),
    Dpt8(i16),
    Dpt9(f32),
    Dpt10(Dpt10Value),
    Dpt11(Dpt11Value),
    Dpt12(u32),
    Dpt13(i32),
    Dpt14(f32),
    Dpt16(String),
    Dpt20(u8),
    Dpt232(Dpt232Value),
    Dpt251(Dpt251Value),
    Raw(Vec<u8>),
}

pub struct KnxDataDecode;

impl KnxDataDecode {
    pub fn get_dpt_number(dpt: &str) -> Option<u32> {
        if dpt.contains('.') {
            let parts: Vec<&str> = dpt.split('.').collect();
            if parts.len() == 2 {
                if let (Ok(p1), Ok(p2)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    return Some(p1 * 1000 + p2);
                }
            }
        }
        dpt.parse::<u32>().ok()
    }

    pub fn fallback_dpt(dpt_num: u32) -> u32 {
        let list = [
            1, 2, 3007, 3008, 4001, 4002, 5, 5001, 5002, 6, 6001, 6010, 6020, 7, 7001, 7002, 7003,
            7004, 7005, 7006, 7007, 7011, 7012, 7013, 8, 9, 10001, 11001, 12001, 12002, 13, 13001,
            13002, 13010, 13011, 13012, 13013, 13014, 13015, 13016, 13100, 14, 15000, 16, 16002,
            20, 20001, 20002, 20003, 20004, 20005, 20006, 20007, 20008, 20011, 20012, 20013, 20014,
            20017, 20020, 20021, 20022, 27001, 28001, 29, 29010, 29011, 29012, 232600, 238600,
            245600, 250600, 251600,
        ];
        if list.contains(&dpt_num) {
            return dpt_num;
        }
        let fallback = dpt_num / 1000;
        if list.contains(&fallback) {
            return fallback;
        }
        dpt_num
    }

    pub fn decode_this(dpt: &str, buffer: &[u8]) -> Result<DptValue, KnxError> {
        let dpt_num = Self::get_dpt_number(dpt).ok_or(KnxError::DPTNotFound)?;
        let resolved = Self::fallback_dpt(dpt_num);

        if buffer.is_empty() {
            return Err(KnxError::InvalidParametersForDpt);
        }

        match resolved {
            1 => {
                let data = 0x3F & buffer[0];
                Ok(DptValue::Dpt1(data != 0))
            }
            2 => {
                let raw = buffer[0] & 0x03;
                let control = (raw >> 1) & 0x01;
                let value = raw & 0x01;
                let description = if control == 0 {
                    if value == 0 {
                        "No control (DPT_Enable_Control)".to_string()
                    } else {
                        "No control (DPT_Ramp_Control)".to_string()
                    }
                } else {
                    if value == 0 {
                        "Control. Function value 0 (DPT_Alarm_Control)".to_string()
                    } else {
                        "Control. Function value 1 (DPT_BinaryValue_Control)".to_string()
                    }
                };
                Ok(DptValue::Dpt2(Dpt2Value {
                    control,
                    value,
                    description,
                }))
            }
            3007 | 3008 => {
                let raw_nibble = buffer[0] & 0x0F;
                let control = (raw_nibble >> 3) & 0x01;
                let step_code = raw_nibble & 0x07;
                let action = if control == 0 {
                    "Decrease".to_string()
                } else {
                    "Increase".to_string()
                };
                let description = if step_code == 0 {
                    "Break".to_string()
                } else {
                    let intervals = 2u32.pow((step_code - 1) as u32);
                    format!("StepCode {} (Intervals: {})", step_code, intervals)
                };
                Ok(DptValue::Dpt3(Dpt3Value {
                    control,
                    step_code,
                    action,
                    description,
                }))
            }
            4001 | 4002 => {
                let val = buffer[0];
                if resolved == 4001 && (val & 0x80) != 0 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                Ok(DptValue::Dpt4(val as char))
            }
            5 => Ok(DptValue::Dpt5(buffer[0])),
            5001 => {
                let val = buffer[0];
                let percent = (val as f32 / 255.0) * 100.0;
                Ok(DptValue::Dpt5001(format!("{:.1}%", percent)))
            }
            5002 => {
                let val = buffer[0];
                let angle = (val as f32 / 255.0) * 360.0;
                Ok(DptValue::Dpt5002(format!("{:.1}ª", angle)))
            }
            6 => Ok(DptValue::Dpt6(buffer[0] as i8)),
            6001 => Ok(DptValue::Dpt6001(format!("{}%", buffer[0] as i8))),
            6010 => Ok(DptValue::Dpt6010(format!(
                "{} counter pulses",
                buffer[0] as i8
            ))),
            6020 => {
                let val = buffer[0];
                let status = if (val >> 3) == 1 {
                    "Activo".to_string()
                } else {
                    "Inactivo".to_string()
                };
                let mode = val & 0x07;
                let mode_text = match mode {
                    0x01 => "Modo 0 activo".to_string(),
                    0x02 => "Modo 1 activo".to_string(),
                    0x04 => "Modo 2 activo".to_string(),
                    _ => "Modo desconocido".to_string(),
                };
                Ok(DptValue::Dpt6020(Dpt6020Value {
                    status,
                    mode: mode_text,
                }))
            }
            7 => {
                if buffer.len() == 1 {
                    Ok(DptValue::Dpt7(buffer[0] as u16))
                } else {
                    Ok(DptValue::Dpt7(BigEndian::read_u16(&buffer[0..2])))
                }
            }
            7001 => {
                let val = BigEndian::read_u16(&buffer[0..2]);
                Ok(DptValue::Dpt7Suffix(format!("{}pulses", val)))
            }
            7002 => {
                let val = BigEndian::read_u16(&buffer[0..2]);
                Ok(DptValue::Dpt7Suffix(format!("{}ms", val)))
            }
            7003 => {
                let val = BigEndian::read_u16(&buffer[0..2]) as f32 / 100.0;
                Ok(DptValue::Dpt7Suffix(format!("{}s", val)))
            }
            7004 => {
                let val = BigEndian::read_u16(&buffer[0..2]) as f32 / 10.0;
                Ok(DptValue::Dpt7Suffix(format!("{}s", val)))
            }
            7005 => {
                let val = BigEndian::read_u16(&buffer[0..2]);
                Ok(DptValue::Dpt7Suffix(format!("{}s", val)))
            }
            7006 => {
                let val = BigEndian::read_u16(&buffer[0..2]);
                Ok(DptValue::Dpt7Suffix(format!("{}min", val)))
            }
            7007 => {
                let val = BigEndian::read_u16(&buffer[0..2]);
                Ok(DptValue::Dpt7Suffix(format!("{}h", val)))
            }
            7011 => {
                let val = BigEndian::read_u16(&buffer[0..2]);
                Ok(DptValue::Dpt7Suffix(format!("{}mm", val)))
            }
            7012 => {
                let val = BigEndian::read_u16(&buffer[0..2]);
                let status = if val == 0 {
                    "No bus power supply functionality available".to_string()
                } else {
                    "".to_string()
                };
                Ok(DptValue::Dpt7012(Dpt7012Value {
                    value: format!("{}mA", val),
                    status,
                }))
            }
            7013 => {
                let val = BigEndian::read_u16(&buffer[0..2]);
                Ok(DptValue::Dpt7Suffix(format!("{}lux", val)))
            }
            8 => {
                if buffer.len() == 1 {
                    Ok(DptValue::Dpt8(buffer[0] as i8 as i16))
                } else {
                    Ok(DptValue::Dpt8(BigEndian::read_i16(&buffer[0..2])))
                }
            }
            9 => {
                if buffer.len() < 2 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let raw = BigEndian::read_u16(&buffer[0..2]);
                if raw == 0x7FFF {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let sign = (raw >> 15) & 0x01;
                let exponent = ((raw >> 11) & 0x0F) as i32;
                let mut mantissa = (raw & 0x07FF) as i32;
                if sign != 0 {
                    mantissa -= 2048;
                }
                let val = 0.01 * mantissa as f32 * 2.0f32.powi(exponent);
                Ok(DptValue::Dpt9(val))
            }
            10001 => {
                if buffer.len() < 3 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let day = (buffer[0] >> 5) & 0x07;
                let hour = buffer[0] & 0x1F;
                let minutes = buffer[1] & 0x3F;
                let seconds = buffer[2] & 0x3F;
                let days = [
                    "No day",
                    "Monday",
                    "Tuesday",
                    "Wednesday",
                    "Thursday",
                    "Friday",
                    "Saturday",
                    "Sunday",
                ];
                let day_name = days
                    .get(day as usize)
                    .copied()
                    .unwrap_or("Unknown")
                    .to_string();

                Ok(DptValue::Dpt10(Dpt10Value {
                    day,
                    day_name,
                    hour,
                    minutes,
                    seconds,
                }))
            }
            11001 => {
                if buffer.len() < 3 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let day = buffer[0] & 0x1F;
                let month = buffer[1] & 0x0F;
                let mut year = (buffer[2] & 0x7F) as u16;
                if year >= 90 {
                    year += 1900;
                } else {
                    year += 2000;
                }
                let formatted = format!("{:02}/{:02}/{:04}", day, month, year);
                Ok(DptValue::Dpt11(Dpt11Value {
                    day,
                    month,
                    year,
                    formatted,
                }))
            }
            12001 | 12002 => {
                if buffer.len() < 4 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                Ok(DptValue::Dpt12(BigEndian::read_u32(&buffer[0..4])))
            }
            13 | 13001 | 13002 | 13010 | 13011 | 13012 | 13013 | 13014 | 13015 | 13016 | 13100 => {
                if buffer.len() < 4 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                Ok(DptValue::Dpt13(BigEndian::read_i32(&buffer[0..4])))
            }
            14 => {
                if buffer.len() < 4 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                Ok(DptValue::Dpt14(BigEndian::read_f32(&buffer[0..4])))
            }
            16 | 16002 | 28001 => {
                let null_pos = buffer.iter().position(|&x| x == 0).unwrap_or(buffer.len());
                let s = String::from_utf8_lossy(&buffer[..null_pos]).into_owned();
                Ok(DptValue::Dpt16(s))
            }
            20 | 20001 | 20002 | 20003 | 20004 | 20005 | 20006 | 20007 | 20008 | 20011 | 20012
            | 20013 | 20014 | 20017 | 20020 | 20021 | 20022 => Ok(DptValue::Dpt20(buffer[0])),
            232600 => {
                if buffer.len() < 3 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                Ok(DptValue::Dpt232(Dpt232Value {
                    r: buffer[0],
                    g: buffer[1],
                    b: buffer[2],
                }))
            }
            251600 => {
                if buffer.len() < 6 {
                    return Err(KnxError::InvalidParametersForDpt);
                }
                let r = buffer[0];
                let g = buffer[1];
                let b = buffer[2];
                let w = buffer[3];
                let validity_bits = buffer[5];
                Ok(DptValue::Dpt251(Dpt251Value {
                    r: Dpt251Val {
                        value: r,
                        valid: (validity_bits & 0x08) != 0,
                    },
                    g: Dpt251Val {
                        value: g,
                        valid: (validity_bits & 0x04) != 0,
                    },
                    b: Dpt251Val {
                        value: b,
                        valid: (validity_bits & 0x02) != 0,
                    },
                    w: Dpt251Val {
                        value: w,
                        valid: (validity_bits & 0x01) != 0,
                    },
                }))
            }
            _ => Ok(DptValue::Raw(buffer.to_vec())),
        }
    }
}
