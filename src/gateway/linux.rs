use super::Gateway;
use crate::interface::MacAddr;
use std::fs::read_to_string;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

const PATH_PROC_NET_ROUTE: &str = "/proc/net/route";
const PATH_PROC_NET_ARP: &str = "/proc/net/arp";

fn convert_hex_ipv4(hex_ip: &str) -> Ipv4Addr {
    if hex_ip.len() != 8 {
        return Ipv4Addr::UNSPECIFIED;
    }
    let o1: u8 = u8::from_str_radix(&hex_ip[6..8], 0x10).unwrap_or(0);
    let o2: u8 = u8::from_str_radix(&hex_ip[4..6], 0x10).unwrap_or(0);
    let o3: u8 = u8::from_str_radix(&hex_ip[2..4], 0x10).unwrap_or(0);
    let o4: u8 = u8::from_str_radix(&hex_ip[0..2], 0x10).unwrap_or(0);
    Ipv4Addr::new(o1, o2, o3, o4)
}

#[allow(dead_code)]
fn convert_hex_ipv6(hex_ip: &str) -> Ipv6Addr {
    if hex_ip.len() != 32 {
        return Ipv6Addr::UNSPECIFIED;
    }
    let h1: u16 = u16::from_str_radix(&hex_ip[0..4], 0x10).unwrap_or(0);
    let h2: u16 = u16::from_str_radix(&hex_ip[4..8], 0x10).unwrap_or(0);
    let h3: u16 = u16::from_str_radix(&hex_ip[8..12], 0x10).unwrap_or(0);
    let h4: u16 = u16::from_str_radix(&hex_ip[12..16], 0x10).unwrap_or(0);
    let h5: u16 = u16::from_str_radix(&hex_ip[16..20], 0x10).unwrap_or(0);
    let h6: u16 = u16::from_str_radix(&hex_ip[20..24], 0x10).unwrap_or(0);
    let h7: u16 = u16::from_str_radix(&hex_ip[24..28], 0x10).unwrap_or(0);
    let h8: u16 = u16::from_str_radix(&hex_ip[28..32], 0x10).unwrap_or(0);
    Ipv6Addr::new(h1, h2, h3, h4, h5, h6, h7, h8)
}

pub fn get_default_gateway(interface_name: String) -> Result<Gateway, String> {
    match super::send_udp_packet() {
        Ok(_) => {}
        Err(e) => return Err(format!("Failed to send UDP packet {}", e)),
    }
    let route_data = read_to_string(PATH_PROC_NET_ROUTE);
    let route_text = match route_data {
        Ok(content) => content,
        Err(_) => String::new(),
    };
    let route_table: Vec<&str> = route_text.trim().split("\n").collect();
    let mut gateway_ip: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    for row in route_table {
        let fields: Vec<&str> = row.split("\t").collect();
        if fields.len() >= 3 {
            if fields[0] == interface_name && fields[2] != "00000000" {
                gateway_ip = IpAddr::V4(convert_hex_ipv4(fields[2]));
            }
        }
    }
    let arp_data = read_to_string(PATH_PROC_NET_ARP);
    let arp_text = match arp_data {
        Ok(content) => content,
        Err(_) => String::new(),
    };
    let arp_table: Vec<&str> = arp_text.trim().split("\n").collect();
    for row in arp_table {
        let mut fields: Vec<&str> = row.split(" ").collect();
        fields.retain(|value| *value != "");
        if fields.len() >= 6 {
            if fields[0] == gateway_ip.to_string() && fields[5] == interface_name {
                let gateway: Gateway = Gateway {
                    mac_addr: MacAddr::from_hex_format(fields[3]),
                    ip_addr: gateway_ip,
                };
                return Ok(gateway);
            }
        }
    }
    Err(String::new())
}
