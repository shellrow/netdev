use std::net::UdpSocket;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use pnet::datalink;
use crate::gateway;

/// Struct of default Network Interface information
pub struct Interface {
    pub index: u32,
    pub name: String,
    pub mac: Option<String>,
    pub ipv4: Vec<Ipv4Addr>,
    pub ipv6: Vec<Ipv6Addr>,
    pub gateway: gateway::Gateway,
}

/// Get default Interface
pub fn get_default_interface()-> Option<Interface> {
    let local_ip = get_local_ipaddr();
    let all_interfaces = datalink::interfaces();
    if let Some(local_ip) = local_ip {
        for iface in all_interfaces{
            for ip in &iface.ips{
                if local_ip == ip.ip().to_string() {
                    let mac_addr: Option<String> = match iface.mac {
                        Some(mac_addr) => Some(mac_addr.to_string()),
                        None => None,
                    };
                    let mut ipv4_vec: Vec<Ipv4Addr> = vec![];
                    let mut ipv6_vec: Vec<Ipv6Addr> = vec![];
                    for ip in &iface.ips {
                        match ip.ip() {
                            IpAddr::V4(ipv4_addr) => {
                                ipv4_vec.push(ipv4_addr);
                            },
                            IpAddr::V6(ipv6_addr) => {
                                ipv6_vec.push(ipv6_addr);
                            },
                        }
                    }
                    let default_gateway = gateway::get_default_gateway();
                    let interface: Interface = Interface{
                        index: iface.index,
                        name: iface.name,
                        mac: mac_addr,
                        ipv4: ipv4_vec,
                        ipv6: ipv6_vec,
                        gateway: default_gateway,
                    };
                    return Some(interface);
                }
            }
        }
        return None;
    }else{
        return None;
    }
}

/// Get default Interface index
pub fn get_default_interface_index() -> Option<u32> {
    let local_ip = get_local_ipaddr();
    let all_interfaces = datalink::interfaces();
    if let Some(local_ip) = local_ip {
        for iface in all_interfaces{
            for ip in iface.ips{
                if local_ip == ip.ip().to_string() {
                    return Some(iface.index)
                }
            }
        }
        return None;
    }else{
        return None;
    }
}

/// Get default Interface name
pub fn get_default_interface_name() -> Option<String> {
    let local_ip = get_local_ipaddr();
    let all_interfaces = datalink::interfaces();
    if let Some(local_ip) = local_ip {
        for iface in all_interfaces{
            for ip in iface.ips{
                if local_ip == ip.ip().to_string() {
                    return Some(iface.name)
                }
            }
        }
        return None;
    }else{
        return None;
    }
}

fn get_local_ipaddr() -> Option<String> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    match socket.connect("1.1.1.1:80") {
        Ok(()) => (),
        Err(_) => return None,
    };

    match socket.local_addr() {
        Ok(addr) => return Some(addr.ip().to_string()),
        Err(_) => return None,
    };
}
