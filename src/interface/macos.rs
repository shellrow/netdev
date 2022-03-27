use std::collections::HashMap;
use system_configuration::network_configuration;
use crate::interface::InterfaceType;

fn get_if_type_from_id(type_id: String) -> InterfaceType {
    match type_id.as_str() {
        "Ethernet" => InterfaceType::Ethernet,
        "IEEE80211" => InterfaceType::Wireless80211,
        "PPP" => InterfaceType::Ppp,
        _ => InterfaceType::Unknown,
    }
}

pub fn get_if_type_map() ->  HashMap<String, InterfaceType> {
    let mut map: HashMap<String, InterfaceType> = HashMap::new();
    let interfaces = network_configuration::get_interfaces();
    for interface in &interfaces {
        let if_name: String = interface.bsd_name().unwrap().to_string();
        let type_id: String = interface.interface_type_string().unwrap().to_string();
        map.insert(if_name, get_if_type_from_id(type_id));
    }
    return map;
}
