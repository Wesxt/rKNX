use rknx::core::control_field::{ControlField, FrameType, Priority};
use rknx::core::control_field_extended::{AddressType, ExtendedControlField, ExtendedFrameFormat};
use rknx::core::device_descriptor_type::DeviceDescriptorType0;
use rknx::core::layers::data::apdu::Apdu;
use rknx::core::layers::data::npdu::Npdu;
use rknx::core::layers::data::tpdu::Tpdu;
use rknx::core::layers::interfaces::apci::Apci;
use rknx::core::layers::interfaces::tpci::Tpci;
use rknx::core::system_status::{Status, StatusValues, SystemStatus, SystemStatusValues};
use rknx::core::knx_add_info_types::*;

#[test]
fn test_apci_pack_unpack() {
    let apci = Apci::new(0x80); // A_GroupValue_Write_Protocol_Data_Unit
    let pack = apci.pack_number();
    assert_eq!(pack, (0, 0x80));

    let unpack = Apci::unpack_number(pack.0, pack.1);
    assert_eq!(unpack, 0x80);
}

#[test]
fn test_apdu_short_data() {
    // 2-byte PDU with short data (<= 6 bits)
    let tpci = Tpci::new(0x00); // T_DATA_GROUP_PDU
    let apci = Apci::new(0x80); // A_GroupValue_Write
    let data = vec![0x03]; // value 3
    let apdu = Apdu::new(tpci, apci, data, true);

    let buffer = apdu.to_buffer();
    // byte0: tpci (0x00) | (apci_high (0) & 3) = 0x00
    // byte1: apci_low (0x80) | (data (3) & 0x3f) = 0x80 | 3 = 0x83
    assert_eq!(buffer, vec![0x00, 0x83]);

    let parsed = Apdu::from_buffer(&buffer).unwrap();
    assert!(parsed.is_short);
    assert_eq!(parsed.data, vec![0x03]);
    assert_eq!(parsed.apci.get_value(), 0x80);
}

#[test]
fn test_apdu_long_data() {
    // Multi-byte PDU
    let tpci = Tpci::new(0x00);
    let apci = Apci::new(0x80);
    let data = vec![0x11, 0x22];
    let apdu = Apdu::new(tpci, apci, data, false);

    let buffer = apdu.to_buffer();
    // byte0: tpci (0x00) | apci_high (0) = 0x00
    // byte1: apci_low (0x80)
    // byte2, 3: payload (0x11, 0x22)
    assert_eq!(buffer, vec![0x00, 0x80, 0x11, 0x22]);

    let parsed = Apdu::from_buffer(&buffer).unwrap();
    assert!(!parsed.is_short);
    assert_eq!(parsed.data, vec![0x11, 0x22]);
    assert_eq!(parsed.apci.get_value(), 0x80);
}

#[test]
fn test_npdu_routing() {
    let tpci = Tpci::new(0x00);
    let apci = Apci::new(0x80);
    let apdu = Apdu::new(tpci, apci, vec![0x03], true);
    
    let tpdu = Tpdu::new(Tpci::new(0x00), apdu, vec![0x03]);
    let npdu = Npdu::new(tpdu, AddressType::Group, 6, 2).unwrap();

    let buffer = npdu.to_buffer();
    // NPCI: AddressType::Group (1) << 7 | hopCount (6) << 4 | length (2) = 0x80 | 0x60 | 0x02 = 0xe2
    // Followed by TPDU buffer [0x00, 0x83]
    assert_eq!(buffer, vec![0xE2, 0x00, 0x83]);

    let parsed = Npdu::from_buffer(&buffer).unwrap();
    assert_eq!(parsed.get_hop_count(), 6);
    assert_eq!(parsed.address_type, AddressType::Group);
    assert_eq!(parsed.length, 2);
    assert_eq!(parsed.tpdu.apdu.data, vec![0x03]);
}

#[test]
fn test_one_to_one_control_field() {
    let mut cf = ControlField::new(0x9C); // 1001 1100
    assert_eq!(cf.get_frame_type(), FrameType::Standard);
    assert_eq!(cf.get_repeat(), false);
    assert_eq!(cf.get_system_broadcast(), true);
    assert_eq!(cf.get_priority(), Priority::Low);
    assert_eq!(cf.get_ack_request(), false);
    assert_eq!(cf.get_confirm(), false);

    cf.set_confirm(true);
    cf.set_priority(Priority::System);
    assert_eq!(cf.get_confirm(), true);
    assert_eq!(cf.get_priority(), Priority::System);
    assert_eq!(cf.get_buffer(), &[0x91]); // 1001 0001
}

#[test]
fn test_one_to_one_extended_control_field() {
    let mut ecf = ExtendedControlField::new(0xE1); // 1110 0001
    assert_eq!(ecf.get_address_type(), AddressType::Group);
    assert_eq!(ecf.get_hop_count(), 6);
    assert_eq!(ecf.get_eff(), ExtendedFrameFormat::PointToPointOrStandardGroupAddressed);

    ecf.set_hop_count(3).unwrap();
    ecf.set_address_type(AddressType::Individual);
    ecf.set_eff(ExtendedFrameFormat::MulticastZoneAddressed11);
    assert_eq!(ecf.to_number(), 0x37); // 0011 0111
}

#[test]
fn test_one_to_one_device_descriptor_type() {
    let desc = DeviceDescriptorType0::new(0x5705);
    assert_eq!(desc.value(), 0x5705);
    assert_eq!(desc.mask_type(), 0x57);
    assert_eq!(desc.medium_type(), 0x05);
    assert_eq!(desc.firmware_type(), 0x07);
    assert_eq!(desc.firmware_version(), 0x05);
    assert_eq!(desc.version(), 0x00);
    assert_eq!(desc.subcode(), 0x05);

    let desc2 = DeviceDescriptorType0::TP1_BCU_1_SYSTEM_1_V2;
    assert_eq!(desc2.value(), 0x0012);
    assert_eq!(desc2.mask_type(), 0x00);
    assert_eq!(desc2.firmware_version(), 0x12);
}

#[test]
fn test_one_to_one_system_status() {
    let ss = SystemStatus::from_byte(0xBA); // 1011 1010
    assert_eq!(ss.get_llm(), true);
    assert_eq!(ss.get_tle(), false);
    assert_eq!(ss.get_ale(), true);
    assert_eq!(ss.get_se(), true);
    assert_eq!(ss.get_ue(), true);
    assert_eq!(ss.get_parity(), true);
    assert_eq!(ss.describe().parity, "even parity");

    let val = SystemStatusValues {
        prog: false,
        llm: true,
        tle: true,
        ale: false,
        se: false,
        ue: true,
        dm: false,
        parity: false,
    };
    let ss_new = SystemStatus::new(val).unwrap();
    assert_eq!(ss_new.get_value(), 0x26); // 0010 0110
}

#[test]
fn test_one_to_one_status() {
    let s = Status::from_byte(0xD5); // 1101 0101
    assert_eq!(s.get_frame_error(), true);
    assert_eq!(s.get_bit_error(), true);
    assert_eq!(s.get_parity_error(), false);
    assert_eq!(s.get_overflow(), true);
    assert_eq!(s.get_lost(), false);
    assert_eq!(s.get_sequence_number(), 5);

    let val = StatusValues {
        frame_error: false,
        bit_error: true,
        parity_error: true,
        overflow: false,
        lost: true,
        sequence_number: 3,
    };
    let s_new = Status::new(val).unwrap();
    assert_eq!(s_new.get_value(), 0x6B); // 0110 1011
}

#[test]
fn test_one_to_one_add_info_types() {
    // 1. PLMediumInfo
    let pl = PLMediumInfo::from_buffer(&[0x01, 0x02, 0x12, 0x34]).unwrap();
    assert_eq!(pl.domain_address, [0x12, 0x34]);
    assert_eq!(pl.get_buffer(), vec![0x01, 0x02, 0x12, 0x34]);

    // 2. RFMediumInformation
    let mut rf = RFMediumInformation::from_buffer(&[0x02, 0x08, 0x93, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x05]).unwrap();
    assert_eq!(rf.get_rssi(), 1);
    assert_eq!(rf.get_battery_state(), true);
    assert_eq!(rf.get_unidir_flag(), true);
    assert_eq!(rf.get_route_last_flag(), true);
    assert_eq!(rf.get_lfn(), 0x05);
    rf.set_rssi(3);
    assert_eq!(rf.get_buffer(), vec![0x02, 0x08, 0xB3, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x05]);

    // 3. BusmonitorStatusInfo
    let bm = BusmonitorStatusInfo::from_buffer(&[0x03, 0x01, 0xD5]).unwrap();
    assert_eq!(bm.status.get_frame_error(), true);
    assert_eq!(bm.status.get_sequence_number(), 5);
    assert_eq!(bm.get_buffer(), vec![0x03, 0x01, 0xD5]);

    // 4. TimestampRelative
    let ts = TimestampRelative::from_buffer(&[0x04, 0x02, 0x12, 0x34]).unwrap();
    assert_eq!(ts.timestamp, 0x1234);
    assert_eq!(ts.get_buffer(), vec![0x04, 0x02, 0x12, 0x34]);

    // 5. TimeDelayUntilSending
    let td = TimeDelayUntilSending::from_buffer(&[0x05, 0x02, 0x56, 0x78]).unwrap();
    assert_eq!(td.delay, 0x5678);
    assert_eq!(td.get_buffer(), vec![0x05, 0x02, 0x56, 0x78]);

    // 6. ExtendedRelativeTimestamp
    let et = ExtendedRelativeTimestamp::from_buffer(&[0x06, 0x04, 0x12, 0x34, 0x56, 0x78]).unwrap();
    assert_eq!(et.timestamp, 0x12345678);
    assert_eq!(et.get_buffer(), vec![0x06, 0x04, 0x12, 0x34, 0x56, 0x78]);

    // 7. BiBatInformation
    let mut bb = BiBatInformation::from_buffer(&[0x07, 0x02, 0xA0, 0x55]).unwrap();
    assert_eq!(bb.get_bibat_ctrl(), 10);
    assert_eq!(bb.bibat_block, 0x55);
    bb.set_bibat_ctrl(5);
    assert_eq!(bb.get_buffer(), vec![0x07, 0x02, 0x50, 0x55]);

    // 8. RFMultiInformation
    let mut rfm = RFMultiInformation::from_buffer(&[0x08, 0x04, 0x11, 0x23, 0x44, 0x55]).unwrap();
    assert_eq!(rfm.transmission_frequency, 0x11);
    assert_eq!(rfm.get_fast_call_channel(), 2);
    assert_eq!(rfm.get_slow_call_channel(), 3);
    assert_eq!(rfm.reception_frequency, 0x55);
    rfm.set_fast_call_channel(8);
    assert_eq!(rfm.get_buffer(), vec![0x08, 0x04, 0x11, 0x83, 0x44, 0x55]);

    // 9. PreambleAndPostamble
    let pap = PreambleAndPostamble::from_buffer(&[0x09, 0x03, 0x12, 0x34, 0x56]).unwrap();
    assert_eq!(pap.preamble_length, 0x1234);
    assert_eq!(pap.postamble_length, 0x56);
    assert_eq!(pap.get_buffer(), vec![0x09, 0x03, 0x12, 0x34, 0x56]);

    // 10. RFFastACKInformation
    let mut rfa = RFFastACKInformation::from_buffer(&[0x0A, 0x04, 0x11, 0x22, 0x33, 0x44]).unwrap();
    assert_eq!(rfa.get_fast_acks(), vec![RfFastAck { status: 0x11, info: 0x22 }, RfFastAck { status: 0x33, info: 0x44 }]);
    rfa.add_fast_ack(RfFastAck { status: 0x55, info: 0x66 });
    assert_eq!(rfa.get_buffer(), vec![0x0A, 0x06, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);

    // 11. ManufacturerSpecificData
    let mut msd = ManufacturerSpecificData::from_buffer(&[0xFE, 0x05, 0x12, 0x34, 0x56, 0xAA, 0xBB]).unwrap();
    assert_eq!(msd.manufacturer_id, 0x1234);
    assert_eq!(msd.subfunction, 0x56);
    assert_eq!(msd.get_data(), &[0xAA, 0xBB]);
    msd.set_data(vec![0xCC]);
    assert_eq!(msd.get_buffer(), vec![0xFE, 0x04, 0x12, 0x34, 0x56, 0xCC]);
}

#[test]
fn test_knxnetip_header_and_structures() {
    use std::net::Ipv4Addr;
    use rknx::core::knxnetip_enum::{HostProtocolCode, ConnectionType, KnxNetIpServiceType, KnxMedium};
    use rknx::core::knxnetip_header::KnxNetIpHeader;
    use rknx::core::knxnetip_structures::{Hpai, Cri, Crd, RoutingBusy, RoutingLostMessage, Dib, DeviceInformationDib, IpConfigDib};

    // 1. Header
    let header = KnxNetIpHeader::new(KnxNetIpServiceType::TunnellingRequest, 100);
    let buf = header.to_buffer();
    assert_eq!(buf, vec![0x06, 0x10, 0x04, 0x20, 0x00, 0x64]);
    let parsed_header = KnxNetIpHeader::from_buffer(&buf).unwrap();
    assert_eq!(parsed_header.service_type, KnxNetIpServiceType::TunnellingRequest);
    assert_eq!(parsed_header.total_length, 100);

    // 2. Hpai
    let ip = Ipv4Addr::new(192, 168, 1, 15);
    let hpai = Hpai::new(HostProtocolCode::Ipv4Udp, ip, 3671);
    let hpai_buf = hpai.to_buffer();
    assert_eq!(hpai_buf, vec![0x08, 0x01, 192, 168, 1, 15, 0x0E, 0x57]);
    let parsed_hpai = Hpai::from_buffer(&hpai_buf).unwrap();
    assert_eq!(parsed_hpai.ip_address, ip);
    assert_eq!(parsed_hpai.port, 3671);

    // 3. Cri & Crd
    let cri = Cri::new(ConnectionType::TunnelConnection, 0x02, Some(0x1105));
    let cri_buf = cri.to_buffer();
    assert_eq!(cri_buf, vec![0x06, 0x04, 0x02, 0x00, 0x11, 0x05]);
    let parsed_cri = Cri::from_buffer(&cri_buf).unwrap();
    assert_eq!(parsed_cri.individual_address, Some(0x1105));

    let crd = Crd::new(ConnectionType::TunnelConnection, 0x1106);
    let crd_buf = crd.to_buffer();
    assert_eq!(crd_buf, vec![0x04, 0x04, 0x11, 0x06]);
    let parsed_crd = Crd::from_buffer(&crd_buf).unwrap();
    assert_eq!(parsed_crd.knx_address, 0x1106);

    // 4. RoutingBusy & RoutingLostMessage
    let busy = RoutingBusy::new(0x01, 200, 0x00);
    assert_eq!(busy.to_buffer(), vec![0x04, 0x01, 0x00, 200, 0x00, 0x00]);

    let lost = RoutingLostMessage::new(0x02, 5);
    assert_eq!(lost.to_buffer(), vec![0x04, 0x02, 0x00, 0x05]);

    // 5. Dib DeviceInfo
    let dib = Dib::DeviceInfo(DeviceInformationDib {
        knx_medium: KnxMedium::Tp1,
        device_status: 1,
        individual_address: 0x1102,
        project_installation_id: 10,
        serial_number: [1, 2, 3, 4, 5, 6],
        routing_multicast_address: Ipv4Addr::new(224, 0, 23, 12),
        mac_address: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        friendly_name: "RustNode".to_string(),
    });
    let dib_buf = dib.to_buffer();
    assert_eq!(dib_buf.len(), 54);
    let parsed_dib = Dib::from_buffer(&dib_buf).unwrap();
    if let Dib::DeviceInfo(dev) = parsed_dib {
        assert_eq!(dev.individual_address, 0x1102);
        assert_eq!(dev.friendly_name, "RustNode");
    } else {
        panic!("DIB type mismatch");
    }

    // 6. Dib IpConfig
    let ip_dib = Dib::IpConfig(IpConfigDib {
        ip_address: Ipv4Addr::new(192, 168, 1, 100),
        subnet_mask: Ipv4Addr::new(255, 255, 255, 0),
        default_gateway: Ipv4Addr::new(192, 168, 1, 1),
        ip_capabilities: 0x03,
        ip_assignment_method: 0x01,
    });
    let ip_dib_buf = ip_dib.to_buffer();
    let parsed_ip_dib = Dib::from_buffer(&ip_dib_buf).unwrap();
    if let Dib::IpConfig(ipc) = parsed_ip_dib {
        assert_eq!(ipc.ip_address, Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(ipc.subnet_mask, Ipv4Addr::new(255, 255, 255, 0));
    } else {
        panic!("DIB type mismatch");
    }
}

#[test]
fn test_cemi_adapter_conversion() {
    use rknx::core::control_field::ControlField;
    use rknx::core::control_field_extended::{ExtendedControlField, AddressType};
    use rknx::core::layers::interfaces::apci::Apci;
    use rknx::core::layers::interfaces::tpci::Tpci;
    use rknx::core::layers::data::apdu::Apdu;
    use rknx::core::layers::data::tpdu::Tpdu;
    use rknx::core::cemi::{Cemi, LData};
    use rknx::core::cemi_adapter::CemiAdapter;

    // Create a CEMI message
    let tpci = Tpci::new(0x00);
    let apci = Apci::new(0x80); // A_GroupValue_Write
    let apdu = Apdu::new(tpci, apci, vec![0x01], true); // ON
    let tpdu = Tpdu::new(Tpci::new(0x00), apdu, vec![0x01]);
    
    let mut control_field2 = ExtendedControlField::new(0);
    control_field2.set_address_type(AddressType::Group);
    let _ = control_field2.set_hop_count(6);

    let cemi = Cemi::LDataInd(LData {
        additional_info: Vec::new(),
        control_field1: ControlField::new(0xbc),
        control_field2,
        source_address: "1.1.1".to_string(),
        destination_address: "1/1/1".to_string(),
        tpdu,
    });

    // Translate to EMI
    let emi = CemiAdapter::cemi_to_emi(&cemi).unwrap();
    let emi_buffer = emi.to_buffer();

    // Verify EMI Frame layout: [MC] [Ctrl] [Src (2)] [Dst (2)] [NPCI] [TPDU]
    // CEMI code 0x29 (ind) -> EMI code 0x29 (ind)
    assert_eq!(emi_buffer[0], 0x29);
    assert_eq!(emi_buffer[1], 0xbc);
    // NPCI byte starts at offset 6: AddressType::Group (1) << 7 | hop (6) << 4 | len (2) = 0xE2
    assert_eq!(emi_buffer[6], 0xE2);

    // Convert back to CEMI
    let cemi_back = CemiAdapter::emi_to_cemi(&emi_buffer).unwrap();
    if let Cemi::LDataInd(ld_back) = cemi_back {
        assert_eq!(ld_back.source_address, "1.1.1");
        assert_eq!(ld_back.destination_address, "1/1/1");
        assert_eq!(ld_back.control_field2.get_hop_count(), 6);
        assert_eq!(ld_back.tpdu.apdu.data, vec![0x01]);
    } else {
        panic!("CEMI conversion type mismatch");
    }
}

#[test]
fn test_new_emi_variants_serialization() {
    use rknx::core::emi::{Emi, TConnectDisconnectEmi};

    let conn_req = Emi::TConnectReq(TConnectDisconnectEmi {
        message_code: 0x43,
        control: 0x00,
        address: "1.1.2".to_string(),
    });
    
    let buffer = conn_req.to_buffer();
    assert_eq!(buffer.len(), 6);
    assert_eq!(buffer[0], 0x43);
    assert_eq!(buffer[1], 0x00);
    
    let parsed = Emi::from_buffer(&buffer).unwrap();
    if let Emi::TConnectReq(tc) = parsed {
        assert_eq!(tc.address, "1.1.2");
        assert_eq!(tc.control, 0x00);
    } else {
        panic!("Expected TConnectReq");
    }

    // 2. M_PropRead.req
    let prop_req = Emi::MPropReadReq(rknx::core::cemi::MProp {
        interface_object_type: 0x0012,
        object_instance: 0x01,
        property_id: 0x02,
        number_of_elements: 0x03,
        start_index: 0x0004,
    });
    let prop_buffer = prop_req.to_buffer();
    assert_eq!(prop_buffer.len(), 7);
    assert_eq!(prop_buffer[0], 0xFC);
    assert_eq!(prop_buffer[3], 0x01);
    assert_eq!(prop_buffer[4], 0x02);

    let prop_parsed = Emi::from_buffer(&prop_buffer).unwrap();
    if let Emi::MPropReadReq(mp) = prop_parsed {
        assert_eq!(mp.interface_object_type, 0x0012);
        assert_eq!(mp.object_instance, 0x01);
        assert_eq!(mp.property_id, 0x02);
        assert_eq!(mp.number_of_elements, 0x03);
        assert_eq!(mp.start_index, 0x0004);
    } else {
        panic!("Expected MPropReadReq");
    }
}

#[test]
fn test_dpt_encoding_decoding() {
    use rknx::core::data::knx_data_decode::{DptValue, KnxDataDecode};
    use rknx::core::data::knx_data_encode::KnxDataEncoder;

    // 1. DPT 1 (Boolean)
    let enc1 = KnxDataEncoder::encode_this("1.001", &DptValue::Dpt1(true)).unwrap();
    assert_eq!(enc1, vec![0x01]);
    let dec1 = KnxDataDecode::decode_this("1.001", &enc1).unwrap();
    assert_eq!(dec1, DptValue::Dpt1(true));

    // 2. DPT 5.001 (Percentage string matching 50%)
    let enc5 = KnxDataEncoder::encode_this("5.001", &DptValue::Dpt5001("50%".to_string())).unwrap();
    // 50% * 255 = 127.5 => rounded to 128 (0x80) or 127 (0x7F) depending on precision
    assert!(enc5 == vec![0x7F] || enc5 == vec![0x80]);
    let dec5 = KnxDataDecode::decode_this("5.001", &enc5).unwrap();
    if let DptValue::Dpt5001(pct) = dec5 {
        assert!(pct.contains("49.8%") || pct.contains("50.2%") || pct.contains("50.0%"));
    } else {
        panic!("DPT 5.001 decode mismatch");
    }

    // 3. DPT 9 (2-byte float Temperature)
    let enc9 = KnxDataEncoder::encode_this("9.001", &DptValue::Dpt9(21.0)).unwrap();
    // 21.0 = 0.01 * 1050 * 2^1. Exponent = 1, Mantissa = 1050 => 0x0C1A
    assert_eq!(enc9, vec![0x0C, 0x1A]);
    let dec9 = KnxDataDecode::decode_this("9.001", &enc9).unwrap();
    if let DptValue::Dpt9(temp) = dec9 {
        assert_eq!(temp, 21.0);
    } else {
        panic!("DPT 9.001 decode mismatch");
    }

    // 4. DPT 232 (RGB Color)
    let rgb = DptValue::Dpt232(rknx::core::data::knx_data_decode::Dpt232Value { r: 255, g: 128, b: 64 });
    let enc_rgb = KnxDataEncoder::encode_this("232.600", &rgb).unwrap();
    assert_eq!(enc_rgb, vec![255, 128, 64]);
    let dec_rgb = KnxDataDecode::decode_this("232.600", &enc_rgb).unwrap();
    assert_eq!(dec_rgb, rgb);
}

