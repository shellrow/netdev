#[cfg(any(target_os = "macos", target_os = "openbsd", target_os = "freebsd", target_os = "netbsd", target_os = "ios"))]
pub(crate) mod unix;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) mod linux;

use std::net::{IpAddr, Ipv4Addr};
use crate::interface::{self, MacAddr, Interface};

/// Structure of default Gateway information
#[derive(Clone, Debug)]
pub struct Gateway {
    pub mac_addr: MacAddr,
    pub ip_addr: IpAddr,
}

impl Gateway {
    pub fn new() -> Gateway {
        Gateway {
            mac_addr: MacAddr::zero(),
            ip_addr: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        }
    }
}

/// Get default Gateway
pub fn get_default_gateway() -> Result<Gateway, String> {
    let local_ip: IpAddr = match interface::get_local_ipaddr(){
        Some(local_ip) => local_ip,
        None => return Err(String::from("Local IP address not found")),
    };
    let interfaces: Vec<Interface> = interface::get_interfaces();
    for iface in interfaces {
        match local_ip {
            IpAddr::V4(local_ipv4) => {
                if iface.ipv4.iter().any(|x| x.addr == local_ipv4) {
                    if let Some(gateway) = iface.gateway {
                        return Ok(gateway);
                    }
                }
            },
            IpAddr::V6(local_ipv6) => {
                if iface.ipv6.iter().any(|x| x.addr == local_ipv6) {
                    if let Some(gateway) = iface.gateway {
                        return Ok(gateway);
                    }
                }
            },
        }
    }
    Err(String::from("Default Gateway not found"))
}

#[cfg(not(target_os="windows"))]
fn send_udp_packet() -> Result<(), String> {
    use std::net::UdpSocket;
    let buf = [0u8; 0];
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => return Err(format!("Failed to create UDP socket {}", e)),
    };
    let dst: &str = "1.1.1.1:80";
    match socket.set_ttl(1) {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to set TTL {}", e)),
    }
    match socket.send_to(&buf, dst) {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to send data {}", e)),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_gateway() {
        println!("{:?}", get_default_gateway());
    }
}
