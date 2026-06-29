use rknx::core::cache::group_address_cache::GroupAddressCache;
use rknx::core::cemi::{Cemi, LData};
use rknx::core::control_field::ControlField;
use rknx::core::control_field_extended::ExtendedControlField;
use rknx::core::layers::data::apdu::Apdu;
use rknx::core::layers::data::tpdu::Tpdu;
use rknx::core::layers::interfaces::apci::{Apci, ApciEnum};
use rknx::core::layers::interfaces::tpci::Tpci;
use rknx::core::data::knx_data_decode::DptValue;
fn create_mock_cemi(dest_addr: &str, apci_enum: ApciEnum, data: Vec<u8>) -> Cemi {
    let tpci = Tpci::new(0x00);
    let apci = Apci::new(apci_enum as u16);
    let apdu = Apdu::new(tpci.clone(), apci, data.clone(), false);
    let tpdu = Tpdu::new(tpci, apdu, data);
    let control_field1 = ControlField::new(0xBC);
    let control_field2 = ExtendedControlField::new(0xE0); // AddressType::Group

    Cemi::LDataInd(LData {
        additional_info: Vec::new(),
        control_field1,
        control_field2,
        source_address: "1.1.1".to_string(),
        destination_address: dest_addr.to_string(),
        tpdu,
    })
}

#[test]
fn test_cache_enable_disable() {
    let cache_lock = GroupAddressCache::get_instance();
    let mut cache = cache_lock.write().unwrap();

    cache.set_enabled(false);
    assert!(!cache.is_enabled());

    cache.set_enabled(true);
    assert!(cache.is_enabled());

    cache.clear();
}

#[test]
fn test_cache_process_and_query() {
    let cache_lock = GroupAddressCache::get_instance();
    let mut cache = cache_lock.write().unwrap();
    cache.set_enabled(true);
    cache.configure(10, 5);

    // Set DPT for address "1/1/1" as "9.001" (Temperature Float)
    cache.set_address_dpt("1/1/1".to_string(), "9.001".to_string());
    assert_eq!(cache.get_address_dpt("1/1/1"), Some("9.001".to_string()));

    // KnxDataEncoder encoding for 9.001 (e.g. 21.0 degrees is [0x0C, 0x1A])
    let encoded_data = vec![0x0C, 0x1A];
    let cemi = create_mock_cemi("1/1/1", ApciEnum::AGroupValueWrite, encoded_data);

    cache.process_cemi(&cemi);

    let query_results = cache.query(&["1/1/1".to_string()], None, None, true);
    assert_eq!(query_results.len(), 1);
    assert_eq!(query_results[0].group_address, "1/1/1");

    // Verify DPT value was auto-decoded
    if let Some(DptValue::Dpt9(temp)) = &query_results[0].decoded_value {
        assert!((temp - 21.0).abs() < 0.1);
    } else {
        panic!("Expected DptValue::Dpt9");
    }

    cache.clear();
}

#[test]
fn test_cache_limitations_and_eviction() {
    let cache_lock = GroupAddressCache::get_instance();
    let mut cache = cache_lock.write().unwrap();
    cache.set_enabled(true);
    cache.configure(3, 2); // Max 3 addresses, max 2 messages per address

    // Process messages for 4 different addresses to force eviction
    let cemi1 = create_mock_cemi("1/1/1", ApciEnum::AGroupValueWrite, vec![0x01]);
    let cemi2 = create_mock_cemi("1/1/2", ApciEnum::AGroupValueWrite, vec![0x02]);
    let cemi3 = create_mock_cemi("1/1/3", ApciEnum::AGroupValueWrite, vec![0x03]);
    let cemi4 = create_mock_cemi("1/1/4", ApciEnum::AGroupValueWrite, vec![0x04]);

    cache.process_cemi(&cemi1);
    cache.process_cemi(&cemi2);
    cache.process_cemi(&cemi3);
    cache.process_cemi(&cemi4);

    // One of them must have been evicted since limit is 3
    let res1 = cache.query(&["1/1/1".to_string()], None, None, true);
    let res2 = cache.query(&["1/1/2".to_string()], None, None, true);
    let res3 = cache.query(&["1/1/3".to_string()], None, None, true);
    let res4 = cache.query(&["1/1/4".to_string()], None, None, true);

    let total_active_addresses =
        (!res1.is_empty() as usize) +
        (!res2.is_empty() as usize) +
        (!res3.is_empty() as usize) +
        (!res4.is_empty() as usize);

    assert_eq!(total_active_addresses, 3);

    // Test message limit per address (max 2 messages)
    let cemi_a = create_mock_cemi("1/1/2", ApciEnum::AGroupValueWrite, vec![0x0A]);
    let cemi_b = create_mock_cemi("1/1/2", ApciEnum::AGroupValueWrite, vec![0x0B]);
    let cemi_c = create_mock_cemi("1/1/2", ApciEnum::AGroupValueWrite, vec![0x0C]);

    cache.process_cemi(&cemi_a);
    cache.process_cemi(&cemi_b);
    cache.process_cemi(&cemi_c);

    let query_all = cache.query(&["1/1/2".to_string()], None, None, false);
    assert_eq!(query_all.len(), 2); // strictly limited to 2
    // The latest message is cemi_c (0x0C)
    assert_eq!(query_all[0].cemi.get_message_code(), cemi_c.get_message_code());

    cache.clear();
}
