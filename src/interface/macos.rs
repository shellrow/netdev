use crate::interface::InterfaceType;
use std::collections::HashMap;
use system_configuration::network_configuration;

fn get_if_type_from_id(type_id: String) -> InterfaceType {
    match type_id.as_str() {
        "Ethernet" => InterfaceType::Ethernet,
        "IEEE80211" => InterfaceType::Wireless80211,
        "PPP" => InterfaceType::Ppp,
        _ => InterfaceType::Unknown,
    }
}

pub fn get_if_type_map() -> HashMap<String, InterfaceType> {
    let mut map: HashMap<String, InterfaceType> = HashMap::new();
    let interfaces = network_configuration::get_interfaces();
    for interface in &interfaces {
        let if_name: String = if let Some(bsd_name) = interface.bsd_name() {
            bsd_name.to_string()
        } else {
            continue;
        };
        let type_id: String = if let Some(type_string) = interface.interface_type_string() {
            type_string.to_string()
        } else {
            continue;
        };
        map.insert(if_name, get_if_type_from_id(type_id));
    }
    return map;
}
