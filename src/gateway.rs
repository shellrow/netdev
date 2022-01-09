use std::net::IpAddr;
use crate::interface::{MacAddr, Interface};
use crate::os;

/// Structure of default Gateway information
#[derive(Clone, Debug)]
pub struct Gateway {
    pub mac_addr: MacAddr,
    pub ip_addr: IpAddr,
}

/// Get default Gateway
pub fn get_default_gateway() -> Result<Gateway, String> {
    let local_ip: IpAddr = match os::get_local_ipaddr(){
        Some(local_ip) => local_ip,
        None => return Err(String::from("Local IP address not found")),
    };
    let interfaces: Vec<Interface> = os::interfaces();
    for iface in interfaces {
        match local_ip {
            IpAddr::V4(local_ipv4) => {
                if iface.ipv4.contains(&local_ipv4) {
                    if let Some(gateway) = iface.gateway {
                        return Ok(gateway);
                    }
                }
            },
            IpAddr::V6(local_ipv6) => {
                if iface.ipv6.contains(&local_ipv6) {
                    if let Some(gateway) = iface.gateway {
                        return Ok(gateway);
                    }
                }
            },
        }
    }
    Err(String::from("Default Gateway not found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_gateway() {
        println!("{:?}", get_default_gateway());
    }
}
