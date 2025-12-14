//! SystemConfiguration::NetworkInterface-related types and functions for macOS.

use crate::interface::types::InterfaceType;
use mac_addr::MacAddr;
use objc2_core_foundation::{CFArray, CFRetained};
use objc2_system_configuration::SCNetworkInterface;
use std::collections::HashMap;
use std::io::Cursor;

const SC_NWIF_PATH: &str = "/Library/Preferences/SystemConfiguration/NetworkInterfaces.plist";

#[derive(Clone, Debug, Default)]
pub(crate) struct SCInterface {
    #[allow(dead_code)]
    pub bsd_name: String,
    #[allow(dead_code)]
    pub mac: Option<MacAddr>,
    pub friendly_name: Option<String>,
    pub sc_type: Option<String>,
    #[allow(dead_code)]
    pub active: Option<bool>,
}

impl SCInterface {
    pub fn if_type(&self) -> Option<InterfaceType> {
        if let Some(sc_type) = &self.sc_type {
            Some(map_sc_interface_type(sc_type))
        } else {
            None
        }
    }
}

fn map_sc_interface_type(type_id: &str) -> InterfaceType {
    match type_id {
        "Bridge" => InterfaceType::Bridge,
        "Ethernet" => InterfaceType::Ethernet,
        "IEEE80211" => InterfaceType::Wireless80211,
        "Loopback" => InterfaceType::Loopback,
        "Modem" => InterfaceType::GenericModem,
        "PPP" => InterfaceType::Ppp,
        "WWAN" => InterfaceType::Wwanpp,
        _ => InterfaceType::Unknown,
    }
}

fn sc_network_interfaces_all() -> CFRetained<CFArray<SCNetworkInterface>> {
    let untyped_ifaces: CFRetained<CFArray> = SCNetworkInterface::all();

    // SAFETY:
    // SCNetworkInterfaceCopyAll() returns a CFArray whose elements are SCNetworkInterfaceRef.
    unsafe {
        let raw = CFRetained::into_raw(untyped_ifaces);
        CFRetained::from_raw(raw.cast())
    }
}

/// Build a map of `BSD name -> SCInterface` using SystemConfiguration.
pub(crate) fn get_sc_interface_map() -> HashMap<String, SCInterface> {
    let mut if_map = HashMap::new();
    let sc_interfaces: CFRetained<CFArray<SCNetworkInterface>> = sc_network_interfaces_all();
    for sc_iface in sc_interfaces.iter() {
        // Key by BSD interface name (e.g. "en0", "bridge0").
        let Some(bsd_name) = sc_iface.bsd_name() else {
            continue;
        };

        let name = bsd_name.to_string();

        let sc_if_type: Option<String> = sc_iface.interface_type().map(|s| s.to_string());

        let friendly_name = sc_iface.localized_display_name().map(|s| s.to_string());

        let mac: Option<MacAddr> = sc_iface
            .hardware_address_string()
            .and_then(|mac_str| Some(MacAddr::from_hex_format(&mac_str.to_string())));

        if_map.insert(
            name.clone(),
            SCInterface {
                bsd_name: name,
                friendly_name,
                sc_type: sc_if_type,
                mac,
                active: None,
            },
        );
    }
    if_map
}

fn load_sc_interfaces_plist_map(bytes: &[u8]) -> HashMap<String, SCInterface> {
    let mut map = HashMap::new();

    let v = match plist::Value::from_reader(Cursor::new(bytes)) {
        Ok(v) => v,
        Err(_) => return map,
    };

    let dict = match v.into_dictionary() {
        Some(d) => d,
        None => return map,
    };

    let interfaces = match dict.get("Interfaces").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return map,
    };

    for it in interfaces {
        let d = match it.as_dictionary() {
            Some(d) => d,
            None => continue,
        };

        let bsd = match d.get("BSD Name").and_then(|v| v.as_string()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };

        let friendly_name = d
            .get("SCNetworkInterfaceInfo")
            .and_then(|v| v.as_dictionary())
            .and_then(|info| info.get("UserDefinedName"))
            .and_then(|v| v.as_string())
            .map(|s| s.to_string());

        let sc_type = d
            .get("SCNetworkInterfaceType")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string());

        let active = d.get("Active").and_then(|v| v.as_boolean());

        let mac = d
            .get("IOMACAddress")
            .and_then(|v| v.as_data())
            .and_then(|data| {
                if data.len() == 6 {
                    let mut a = [0u8; 6];
                    a.copy_from_slice(data);
                    Some(MacAddr::from_octets(a))
                } else {
                    None
                }
            });

        map.insert(
            bsd.clone(),
            SCInterface {
                bsd_name: bsd,
                friendly_name,
                sc_type,
                mac,
                active,
            },
        );
    }

    map
}

/// Read and parse the NetworkInterfaces.plist file into a map of `BSD name -> SCInterface`.
pub(crate) fn read_sc_interfaces_plist_map() -> std::io::Result<HashMap<String, SCInterface>> {
    let bytes = std::fs::read(SC_NWIF_PATH)?;
    Ok(load_sc_interfaces_plist_map(&bytes))
}
