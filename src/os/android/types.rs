use crate::interface::types::InterfaceType;

pub fn guess_type_by_name(name: &str) -> Option<InterfaceType> {
    let n = name.as_bytes();

    // Wi-Fi: wlan0 / wlan1 / wifi0
    if n.starts_with(b"wlan") || n.starts_with(b"wifi") {
        return Some(InterfaceType::Wireless80211);
    }
    // Cellular: rmnet_data0 / rmnet0 / ccmni0 / pdp0
    if n.starts_with(b"rmnet") || n.starts_with(b"ccmni") || n.starts_with(b"pdp") {
        return Some(InterfaceType::Wwanpp);
    }
    // Tunnel: tun0 / tap0 / ipsec0 / clat4
    if n.starts_with(b"tun")
        || n.starts_with(b"tap")
        || n.starts_with(b"ipsec")
        || n.starts_with(b"clat")
    {
        return Some(InterfaceType::Tunnel);
    }
    // Bridge / veth
    if n.starts_with(b"br-") || n.starts_with(b"bridge") {
        return Some(InterfaceType::Bridge);
    }
    if n.starts_with(b"veth") {
        return Some(InterfaceType::ProprietaryVirtual);
    }

    None
}
