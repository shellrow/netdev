use crate::interface::InterfaceType;
use std::fs::read_to_string;
use std::{collections::HashMap, net::IpAddr};
use system_configuration::network_configuration;

const PATH_RESOLV_CONF: &str = "/etc/resolv.conf";

fn get_if_type_from_id(type_id: String) -> InterfaceType {
    match type_id.as_str() {
        "Ethernet" => InterfaceType::Ethernet,
        "IEEE80211" => InterfaceType::Wireless80211,
        "PPP" => InterfaceType::Ppp,
        "Bridge" => InterfaceType::Bridge,
        _ => InterfaceType::Unknown,
    }
}

#[derive(Debug)]
pub struct SCInterface {
    pub name: String,
    pub friendly_name: Option<String>,
    pub interface_type: InterfaceType,
}

pub fn get_if_type_map() -> HashMap<String, SCInterface> {
    let mut map: HashMap<String, SCInterface> = HashMap::new();
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
        let friendly_name: Option<String> = if let Some(name) = interface.display_name() {
            Some(name.to_string())
        } else {
            None
        };
        let sc_if = SCInterface {
            name: if_name.clone(),
            friendly_name: friendly_name,
            interface_type: get_if_type_from_id(type_id),
        };
        map.insert(if_name, sc_if);
    }
    return map;
}

pub fn get_system_dns_conf() -> Vec<IpAddr> {
    let r = read_to_string(PATH_RESOLV_CONF);
    match r {
        Ok(content) => {
            let conf_lines: Vec<&str> = content.trim().split("\n").collect();
            let mut dns_servers = Vec::new();
            for line in conf_lines {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 2 {
                    // field [0]: Configuration type (e.g., "nameserver", "domain", "search")
                    // field [1]: Corresponding value (e.g., IP address, domain name)
                    if fields[0] == "nameserver" {
                        if let Ok(ip) = fields[1].parse::<IpAddr>() {
                            dns_servers.push(ip);
                        } else {
                            eprintln!("Invalid IP address format: {}", fields[1]);
                        }
                    }
                }
            }
            dns_servers
        }
        Err(_) => {
            return Vec::new();
        }
    }
}
