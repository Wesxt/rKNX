use rknx::utils::knx_helper::KnxHelper;

#[test]
fn test_address_to_buffer_and_back_individual() {
    // Individual Address: 1.1.100
    let raw = KnxHelper::get_address_from_string("1.1.100").unwrap();
    assert_eq!(raw, [0x11, 0x64]);

    let str_addr = KnxHelper::get_address_to_string(&raw, ".", false).unwrap();
    assert_eq!(str_addr, "1.1.100");
}

#[test]
fn test_address_to_buffer_and_back_group_3level() {
    // 3-Level Group Address: 1/2/3
    let raw = KnxHelper::get_address_from_string("1/2/3").unwrap();
    assert_eq!(raw, [0x0a, 0x03]);

    let str_addr = KnxHelper::get_address_to_string(&raw, "/", true).unwrap();
    assert_eq!(str_addr, "1/2/3");
}

#[test]
fn test_address_to_buffer_and_back_group_2level() {
    // 2-Level Group Address: 1/2047
    let raw = KnxHelper::get_address_from_string("1/2047").unwrap();
    assert_eq!(raw, [0x0f, 0xff]);

    let str_addr = KnxHelper::get_address_to_string(&raw, "/", false).unwrap();
    assert_eq!(str_addr, "1/2047");
}

#[test]
fn test_address_validation() {
    assert!(KnxHelper::is_valid_group_address("31/7/255"));
    assert!(!KnxHelper::is_valid_group_address("32/7/255"));
    assert!(!KnxHelper::is_valid_group_address("31/8/255"));
    assert!(!KnxHelper::is_valid_group_address("31/7/260"));

    assert!(KnxHelper::is_valid_individual_address("15.15.255"));
    assert!(!KnxHelper::is_valid_individual_address("16.15.255"));
    assert!(!KnxHelper::is_valid_individual_address("15.16.255"));
    assert!(!KnxHelper::is_valid_individual_address("15.15.256"));
}

#[test]
fn test_write_data_short() {
    // Test short data optimization (DPT1/2/3 <= 6 bits)
    // datagram starts with APCI command byte (e.g. 0x80 for GroupValue_Write)
    let mut datagram = [0x80];
    let data = [0x03]; // value 3
    KnxHelper::write_data(&mut datagram, &data, 0, true);
    // 0x80 & 0xc0 = 0x80. 0x03 & 0x3f = 0x03. 0x80 | 0x03 = 0x83.
    assert_eq!(datagram[0], 0x83);
}

#[test]
fn test_write_data_long() {
    // Test multi-byte standard data
    let mut datagram = [0x80, 0x00, 0x00, 0x00];
    let data = [0x12, 0x34];
    KnxHelper::write_data(&mut datagram, &data, 0, false);
    // datagram[0] remains 0x80. data is written starting at datagram[1]
    assert_eq!(datagram, [0x80, 0x12, 0x34, 0x00]);
}
