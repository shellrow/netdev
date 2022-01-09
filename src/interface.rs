use std::net::UdpSocket;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use crate::gateway::{self, Gateway};
use crate::os;

/// Struct of MAC address
#[derive(Clone, Debug)]
pub struct MacAddr(u8, u8, u8, u8, u8, u8);

impl MacAddr {
    /// Construct a new MacAddr struct from the given octets
    pub fn new(octets: [u8; 6]) -> MacAddr {
        MacAddr(octets[0], octets[1], octets[2], octets[3], octets[4], octets[5])
    }
    /// Returns an array of MAC address octets
    pub fn octets(&self) -> [u8; 6] {
        [self.0,self.1,self.2,self.3,self.4,self.5]
    }
    /// Return a formatted string of MAC address
    pub fn address(&self) -> String {
        format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", self.0,self.1,self.2,self.3,self.4,self.5)
    }
    pub fn zero() -> MacAddr {
        MacAddr(0,0,0,0,0,0)
    }
}

impl std::fmt::Display for MacAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _ = write!(f,"{:<02x}:{:<02x}:{:<02x}:{:<02x}:{:<02x}:{:<02x}",self.0,self.1,self.2,self.3,self.4,self.5);
        Ok(())   
    }
}

/// Struct of default Network Interface information
#[derive(Clone, Debug)]
pub struct Interface {
    pub index: u32,
    pub name: String,
    pub description: Option<String>,
    pub mac_addr: Option<MacAddr>,
    pub ipv4: Vec<Ipv4Addr>,
    pub ipv6: Vec<Ipv6Addr>,
    pub gateway: Option<Gateway>,
}

#[cfg(target_os = "windows")]
pub fn get_default_interface() -> Result<Interface, String> {
    let local_ip: IpAddr = match os::get_local_ipaddr(){
        Some(local_ip) => local_ip,
        None => return Err(String::from("Local IP address not found")),
    };
    let interfaces: Vec<Interface> = os::get_interfaces();
    for iface in interfaces {
        match local_ip {
            IpAddr::V4(local_ipv4) => {
                if iface.ipv4.contains(&local_ipv4) {
                    return Ok(iface);
                }
            },
            IpAddr::V6(local_ipv6) => {
                if iface.ipv6.contains(&local_ipv6) {
                    return Ok(iface);
                }
            },
        }
    }
    Err(String::from("Default Interface not found"))
}

#[cfg(not(target_os="windows"))]
/// Get default Interface
pub fn get_default_interface() -> Result<Interface, String> {
    let local_ip = get_local_ipaddr();
    let interfaces = pnet_datalink::interfaces();
    if let Some(local_ip) = local_ip {
        for iface in interfaces{
            for ip in &iface.ips{
                if local_ip == ip.ip().to_string() {
                    let mac_addr: Option<MacAddr> = match iface.mac {
                        Some(mac_addr) => Some(MacAddr::new(mac_addr.octets())),
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
                    let default_gateway: Option<Gateway> = match gateway::get_default_gateway() {
                        Ok(gateway) => Some(gateway),
                        Err(_) => None,
                    };
                    let interface: Interface = Interface{
                        index: iface.index,
                        name: iface.name,
                        description: None,
                        mac_addr: mac_addr,
                        ipv4: ipv4_vec,
                        ipv6: ipv6_vec,
                        gateway: default_gateway,
                    };
                    return Ok(interface);
                }
            }
        }
        return Err(String::from(""));
    }else{
        return Err(String::from(""));
    }
}

/// Get default Interface index
pub fn get_default_interface_index() -> Option<u32> {
    let local_ip = get_local_ipaddr();
    let interfaces = pnet_datalink::interfaces();
    if let Some(local_ip) = local_ip {
        for iface in interfaces {
            for ip in iface.ips {
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
    let interfaces = pnet_datalink::interfaces();
    if let Some(local_ip) = local_ip {
        for iface in interfaces {
            for ip in iface.ips {
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
