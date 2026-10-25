use crate::net::device::NetworkDevice;
use crate::net::mac::MacAddr;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::net::{Ipv4Addr, Ipv6Addr};

const PATH_PROC_NET_ROUTE: &str = "/proc/net/route";
const PATH_PROC_NET_IPV6_ROUTE: &str = "/proc/net/ipv6_route";
const PATH_PROC_NET_ARP: &str = "/proc/net/arp";

fn convert_hex_ipv4(hex_ip: &str) -> Option<Ipv4Addr> {
    if hex_ip.len() != 8 {
        return None;
    }
    // /proc/net/route prints the native-endian integer backing the IPv4 address.
    u32::from_str_radix(hex_ip, 16)
        .ok()
        .map(|value| Ipv4Addr::from(value.to_ne_bytes()))
}

fn convert_hex_ipv6(hex_ip: &str) -> Option<Ipv6Addr> {
    if hex_ip.len() != 32 {
        return None;
    }
    u128::from_str_radix(hex_ip, 16).ok().map(Ipv6Addr::from)
}

fn get_arp_map() -> HashMap<(String, Ipv4Addr), MacAddr> {
    read_to_string(PATH_PROC_NET_ARP)
        .map(|text| parse_arp_map(&text))
        .unwrap_or_default()
}

fn parse_arp_map(text: &str) -> HashMap<(String, Ipv4Addr), MacAddr> {
    let mut map = HashMap::new();
    for row in text.lines() {
        let fields: Vec<_> = row.split_whitespace().collect();
        if fields.len() < 6 {
            continue;
        }
        let Ok(flags) = u32::from_str_radix(fields[2].trim_start_matches("0x"), 16) else {
            continue;
        };
        if flags & 2 == 0 {
            continue;
        }
        if let Ok(ip) = fields[0].parse() {
            map.insert(
                (fields[5].to_owned(), ip),
                MacAddr::from_hex_format(fields[3]),
            );
        }
    }
    map
}

fn get_ipv4_gateway_map() -> HashMap<String, Ipv4Addr> {
    read_to_string(PATH_PROC_NET_ROUTE)
        .map(|text| parse_ipv4_gateway_map(&text))
        .unwrap_or_default()
}

fn parse_ipv4_gateway_map(text: &str) -> HashMap<String, Ipv4Addr> {
    let mut gateways = HashMap::new();
    for row in text.lines() {
        let fields: Vec<_> = row.split_whitespace().collect();
        if fields.len() < 8 || fields[1] != "00000000" || fields[7] != "00000000" {
            continue;
        }
        let Ok(flags) = u32::from_str_radix(fields[3], 16) else {
            continue;
        };
        // Require an active gateway route, excluding rejected routes.
        if flags & 3 != 3 || flags & 0x200 != 0 {
            continue;
        }
        if let Some(gateway) = convert_hex_ipv4(fields[2]).filter(|ip| !ip.is_unspecified()) {
            gateways.insert(fields[0].to_owned(), gateway);
        }
    }
    gateways
}

fn get_ipv6_gateway_map() -> HashMap<String, Ipv6Addr> {
    read_to_string(PATH_PROC_NET_IPV6_ROUTE)
        .map(|text| parse_ipv6_gateway_map(&text))
        .unwrap_or_default()
}

fn parse_ipv6_gateway_map(text: &str) -> HashMap<String, Ipv6Addr> {
    let mut gateways = HashMap::new();
    for row in text.lines() {
        let fields: Vec<_> = row.split_whitespace().collect();
        if fields.len() < 10 || fields[0] != "00000000000000000000000000000000" || fields[1] != "00"
        {
            continue;
        }
        let Ok(flags) = u32::from_str_radix(fields[8], 16) else {
            continue;
        };
        // Apply the same active gateway and reject checks as the IPv4 table.
        if flags & 3 != 3 || flags & 0x200 != 0 {
            continue;
        }
        if let Some(gateway) = convert_hex_ipv6(fields[4]).filter(|ip| !ip.is_unspecified()) {
            gateways.insert(fields[9].to_owned(), gateway);
        }
    }
    gateways
}

pub fn get_gateway_map() -> HashMap<String, NetworkDevice> {
    let mut gateway_map: HashMap<String, NetworkDevice> = HashMap::new();
    let mut arp_map = get_arp_map();
    for (if_name, ipv4_addr) in get_ipv4_gateway_map() {
        let mac = arp_map.remove(&(if_name.clone(), ipv4_addr));
        let gateway = gateway_map.entry(if_name).or_insert(NetworkDevice::new());
        if let Some(mac_addr) = mac {
            gateway.mac_addr = mac_addr;
        }
        gateway.ipv4.push(ipv4_addr);
    }
    for (if_name, ipv6_addr) in get_ipv6_gateway_map() {
        let gateway = gateway_map.entry(if_name).or_insert(NetworkDevice::new());
        gateway.ipv6.push(ipv6_addr);
    }
    gateway_map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arp_entries_are_scoped_to_their_interface() {
        let map = parse_arp_map(
            "192.168.1.1 0x1 0x2 00:01:02:03:04:05 * eth0\n192.168.1.1 0x1 0x2 06:07:08:09:0a:0b * eth1\n192.168.1.1 0x1 0x0 00:00:00:00:00:00 * eth2",
        );
        let ip = "192.168.1.1".parse().unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(
            map[&("eth0".into(), ip)],
            MacAddr::from_octets([0, 1, 2, 3, 4, 5])
        );
        assert_eq!(
            map[&("eth1".into(), ip)],
            MacAddr::from_octets([6, 7, 8, 9, 10, 11])
        );
    }

    #[test]
    fn ipv6_only_accepts_active_default_gateway_routes() {
        let zero = "00000000000000000000000000000000";
        let gateway = "fe800000000000000000000000000001";
        let rejected_gateway = "fe800000000000000000000000000002";
        let text = format!(
            "{zero} 00 {zero} 00 {gateway} 00000064 00000000 00000000 00000003 eth0\n             {zero} 00 {zero} 00 {rejected_gateway} 00000064 00000000 00000000 00000203 eth0\n             {zero} 00 {zero} 00 {gateway} 00000064 00000000 00000000 00000002 eth1\n             {zero} 40 {zero} 00 {gateway} 00000064 00000000 00000000 00000003 eth2\n             {zero} 00 {zero} 00 {zero} 00000064 00000000 00000000 00000003 eth3\n             {zero} 00 {zero} 00 invalid 00000064 00000000 00000000 00000003 eth4"
        );
        assert_eq!(
            parse_ipv6_gateway_map(&text),
            HashMap::from([("eth0".into(), "fe80::1".parse().unwrap())])
        );
    }

    #[test]
    fn ipv6_hex_parser_rejects_invalid_input() {
        assert_eq!(
            convert_hex_ipv6("fe800000000000000000000000000001"),
            Some("fe80::1".parse().unwrap())
        );
        assert_eq!(convert_hex_ipv6(&"z".repeat(32)), None);
        assert_eq!(convert_hex_ipv6(&"\u{3042}".repeat(10)), None);
    }

    #[test]
    fn only_accepts_active_default_gateway_routes() {
        let gateway = Ipv4Addr::new(192, 0, 2, 1);
        let hex = format!("{:08X}", u32::from_ne_bytes(gateway.octets()));
        let text = format!(
            "Iface Destination Gateway Flags RefCnt Use Metric Mask
eth0 00000000 {hex} 0003 0 0 100 00000000
eth0 0000000A 0100000A 0003 0 0 0 000000FF
eth1 00000000 {hex} 0002 0 0 0 00000000
eth2 00000000 {hex} 0203 0 0 0 00000000
eth3 00000000 00000000 0003 0 0 0 00000000
eth4 00000000 invalid! 0003 0 0 0 00000000
"
        );
        assert_eq!(
            parse_ipv4_gateway_map(&text),
            HashMap::from([("eth0".into(), gateway)])
        );
    }
}
