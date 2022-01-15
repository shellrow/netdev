#[cfg(not(target_os="windows"))]
mod unix;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use crate::gateway::{Gateway};
use crate::os;

/// Structure of MAC address
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

/// Structure of Network Interface information
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

/// Get default Network Interface
pub fn get_default_interface() -> Result<Interface, String> {
    let local_ip: IpAddr = match os::get_local_ipaddr(){
        Some(local_ip) => local_ip,
        None => return Err(String::from("Local IP address not found")),
    };
    let interfaces: Vec<Interface> = os::interfaces();
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

/// Get default Network Interface index
pub fn get_default_interface_index() -> Option<u32> {
    os::default_interface_index()
}

/// Get default Network Interface name
pub fn get_default_interface_name() -> Option<String> {
    os::default_interface_name()
}

/// Get a list of available Network Interfaces
pub fn get_interfaces() -> Vec<Interface> {
    os::interfaces()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_interfaces() {
        let interfaces = get_interfaces();
        for interface in interfaces {
            println!("{:#?}", interface);
        }
    }
    #[test]
    fn test_default_interface() {
        println!("{:#?}", get_default_interface());
    }
    #[test]
    fn test_default_interface_index() {
        println!("{:?}", get_default_interface_index());
    }
    #[test]
    fn test_default_interface_name() {
        println!("{:?}", get_default_interface_name());
    }
}
