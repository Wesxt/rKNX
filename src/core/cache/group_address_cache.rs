use crate::core::cemi::Cemi;
use crate::core::data::knx_data_decode::{DptValue, KnxDataDecode};
use crate::core::layers::interfaces::apci::ApciEnum;
use std::collections::{HashMap, VecDeque};
use std::sync::{OnceLock, RwLock};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub cemi: Cemi,
    pub timestamp: SystemTime,
    pub group_address: String,
    pub decoded_value: Option<DptValue>,
}

pub struct GroupAddressCache {
    enabled: bool,
    max_addresses: usize,
    max_messages_per_address: usize,
    cache: HashMap<String, VecDeque<CacheEntry>>,
    dpt_config: HashMap<String, String>,
    update_tx: tokio::sync::broadcast::Sender<CacheEntry>,
}

impl GroupAddressCache {
    fn new() -> Self {
        let (update_tx, _) = tokio::sync::broadcast::channel(100);
        Self {
            enabled: false,
            max_addresses: 65535,
            max_messages_per_address: 10,
            cache: HashMap::new(),
            dpt_config: HashMap::new(),
            update_tx,
        }
    }

    pub fn get_instance() -> &'static RwLock<GroupAddressCache> {
        static INSTANCE: OnceLock<RwLock<GroupAddressCache>> = OnceLock::new();
        INSTANCE.get_or_init(|| RwLock::new(GroupAddressCache::new()))
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<CacheEntry> {
        self.update_tx.subscribe()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn configure(&mut self, max_addresses: usize, max_messages_per_address: usize) {
        self.max_addresses = max_addresses;
        self.max_messages_per_address = max_messages_per_address;
    }

    pub fn set_address_dpt(&mut self, address: String, dpt: String) {
        self.dpt_config.insert(address, dpt);
    }

    pub fn get_address_dpt(&self, address: &str) -> Option<String> {
        self.dpt_config.get(address).cloned()
    }

    pub fn encode_value(&self, address: &str, value: &DptValue) -> Option<Vec<u8>> {
        let dpt = self.dpt_config.get(address)?;
        crate::core::data::knx_data_encode::KnxDataEncoder::encode_this(dpt, value).ok()
    }

    pub fn process_cemi(&mut self, cemi: &Cemi) {
        if !self.enabled {
            return;
        }

        let ldata = match cemi {
            Cemi::LDataReq(ld) | Cemi::LDataCon(ld) | Cemi::LDataInd(ld) => ld,
            _ => return,
        };

        if ldata.control_field2.get_address_type()
            != crate::core::control_field_extended::AddressType::Group
        {
            return;
        }

        let apci_val = ldata.tpdu.apdu.apci.get_value();
        let is_valid_apci = apci_val == ApciEnum::AGroupValueRead as u16
            || apci_val == ApciEnum::AGroupValueResponse as u16
            || apci_val == ApciEnum::AGroupValueWrite as u16;

        if !is_valid_apci {
            return;
        }

        let target_address = ldata.destination_address.clone();

        if !self.cache.contains_key(&target_address) {
            if self.cache.len() >= self.max_addresses {
                let first_key = self.cache.keys().next().cloned();
                if let Some(k) = first_key {
                    self.cache.remove(&k);
                }
            }
            self.cache.insert(target_address.clone(), VecDeque::new());
        }

        let entries = self.cache.get_mut(&target_address).unwrap();

        let decoded_value = if let Some(dpt) = self.dpt_config.get(&target_address) {
            KnxDataDecode::decode_this(dpt, &ldata.tpdu.apdu.data).ok()
        } else {
            None
        };

        let entry = CacheEntry {
            cemi: cemi.clone(),
            timestamp: SystemTime::now(),
            group_address: target_address,
            decoded_value,
        };

        let _ = self.update_tx.send(entry.clone());

        entries.push_front(entry);

        if entries.len() > self.max_messages_per_address {
            entries.truncate(self.max_messages_per_address);
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn delete_address(&mut self, address: &str) -> bool {
        self.cache.remove(address).is_some()
    }

    pub fn query(
        &self,
        addresses: &[String],
        start_date: Option<SystemTime>,
        end_date: Option<SystemTime>,
        return_only_latest: bool,
    ) -> Vec<CacheEntry> {
        let end_time = end_date.unwrap_or_else(SystemTime::now);
        let mut results = Vec::new();

        for address in addresses {
            let entries = match self.cache.get(address) {
                Some(e) => e,
                None => continue,
            };
            if entries.is_empty() {
                continue;
            }

            if let Some(start_time) = start_date {
                let mut valid_entries = entries
                    .iter()
                    .filter(|e| e.timestamp >= start_time && e.timestamp <= end_time);
                if return_only_latest {
                    if let Some(latest) = valid_entries.next() {
                        results.push(latest.clone());
                    }
                } else {
                    results.extend(valid_entries.cloned());
                }
            } else {
                if return_only_latest {
                    results.push(entries[0].clone());
                } else {
                    let valid_entries = entries.iter().filter(|e| e.timestamp <= end_time);
                    results.extend(valid_entries.cloned());
                }
            }
        }

        results
    }
}
