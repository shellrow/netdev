mod shared;
pub use self::shared::*;

mod types;
pub use self::types::*;

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "ios",
    target_os = "android"
))]
mod unix;
#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "ios",
    target_os = "android"
))]
use self::unix::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows::*;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;

#[cfg(target_os = "android")]
mod android;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod macos;

use crate::gateway::Gateway;
use crate::ip::{Ipv4Net, Ipv6Net};
use std::net::IpAddr;

use crate::sys;

/// Structure of MAC address
#[derive(Clone, Debug)]
pub struct MacAddr(u8, u8, u8, u8, u8, u8);

impl MacAddr {
    /// Construct a new MacAddr instance from the given octets
    pub fn new(octets: [u8; 6]) -> MacAddr {
        MacAddr(
            octets[0], octets[1], octets[2], octets[3], octets[4], octets[5],
        )
    }
    /// Returns an array of MAC address octets
    pub fn octets(&self) -> [u8; 6] {
        [self.0, self.1, self.2, self.3, self.4, self.5]
    }
    /// Return a formatted string of MAC address
    pub fn address(&self) -> String {
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0, self.1, self.2, self.3, self.4, self.5
        )
    }
    /// Construct an all-zero MacAddr instance
    pub fn zero() -> MacAddr {
        MacAddr(0, 0, 0, 0, 0, 0)
    }
    /// Construct a new MacAddr instance from a colon-separated string of hex format
    pub fn from_hex_format(hex_mac_addr: &str) -> MacAddr {
        if hex_mac_addr.len() != 17 {
            return MacAddr(0, 0, 0, 0, 0, 0);
        }
        let fields: Vec<&str> = hex_mac_addr.split(":").collect();
        let o1: u8 = u8::from_str_radix(&fields[0], 0x10).unwrap_or(0);
        let o2: u8 = u8::from_str_radix(&fields[1], 0x10).unwrap_or(0);
        let o3: u8 = u8::from_str_radix(&fields[2], 0x10).unwrap_or(0);
        let o4: u8 = u8::from_str_radix(&fields[3], 0x10).unwrap_or(0);
        let o5: u8 = u8::from_str_radix(&fields[4], 0x10).unwrap_or(0);
        let o6: u8 = u8::from_str_radix(&fields[5], 0x10).unwrap_or(0);
        MacAddr(o1, o2, o3, o4, o5, o6)
    }
}

impl std::fmt::Display for MacAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _ = write!(
            f,
            "{:<02x}:{:<02x}:{:<02x}:{:<02x}:{:<02x}:{:<02x}",
            self.0, self.1, self.2, self.3, self.4, self.5
        );
        Ok(())
    }
}

/// Structure of Network Interface information
#[derive(Clone, Debug)]
pub struct Interface {
    /// Index of network interface
    pub index: u32,
    /// Name of network interface
    pub name: String,
    /// Friendly Name of network interface
    pub friendly_name: Option<String>,
    /// Description of the network interface
    pub description: Option<String>,
    /// Interface Type
    pub if_type: InterfaceType,
    /// MAC address of network interface
    pub mac_addr: Option<MacAddr>,
    /// List of Ipv4Net for the network interface
    pub ipv4: Vec<Ipv4Net>,
    /// List of Ipv6Net for the network interface
    pub ipv6: Vec<Ipv6Net>,
    /// Flags for the network interface (OS Specific)
    pub flags: u32,
    /// Speed in bits per second of the transmit for the network interface
    pub transmit_speed: Option<u64>,
    /// Speed in bits per second of the receive for the network interface
    pub receive_speed: Option<u64>,
    /// Default gateway for the network interface
    pub gateway: Option<Gateway>,
}

impl Interface {
    /// Check if the network interface is up
    pub fn is_up(&self) -> bool {
        self.flags & (sys::IFF_UP as u32) != 0
    }
    /// Check if the network interface is a loopback interface
    pub fn is_loopback(&self) -> bool {
        self.flags & (sys::IFF_LOOPBACK as u32) != 0
    }
    /// Check if the network interface is a point-to-point interface
    pub fn is_point_to_point(&self) -> bool {
        self.flags & (sys::IFF_POINTOPOINT as u32) != 0
    }
    /// Check if the network interface is a multicast interface
    pub fn is_multicast(&self) -> bool {
        self.flags & (sys::IFF_MULTICAST as u32) != 0
    }
    /// Check if the network interface is a broadcast interface
    pub fn is_broadcast(&self) -> bool {
        self.flags & (sys::IFF_BROADCAST as u32) != 0
    }
}

/// Get default Network Interface
pub fn get_default_interface() -> Result<Interface, String> {
    let local_ip: IpAddr = match get_local_ipaddr() {
        Some(local_ip) => local_ip,
        None => return Err(String::from("Local IP address not found")),
    };
    let interfaces: Vec<Interface> = interfaces();
    for iface in interfaces {
        match local_ip {
            IpAddr::V4(local_ipv4) => {
                if iface.ipv4.iter().any(|x| x.addr == local_ipv4) {
                    return Ok(iface);
                }
            }
            IpAddr::V6(local_ipv6) => {
                if iface.ipv6.iter().any(|x| x.addr == local_ipv6) {
                    return Ok(iface);
                }
            }
        }
    }
    Err(String::from("Default Interface not found"))
}

/// Get default Network Interface index
pub fn get_default_interface_index() -> Option<u32> {
    let local_ip: IpAddr = match get_local_ipaddr() {
        Some(local_ip) => local_ip,
        None => return None,
    };
    let interfaces = interfaces();
    for iface in interfaces {
        match local_ip {
            IpAddr::V4(local_ipv4) => {
                if iface.ipv4.iter().any(|x| x.addr == local_ipv4) {
                    return Some(iface.index);
                }
            }
            IpAddr::V6(local_ipv6) => {
                if iface.ipv6.iter().any(|x| x.addr == local_ipv6) {
                    return Some(iface.index);
                }
            }
        }
    }
    None
}

/// Get default Network Interface name
pub fn get_default_interface_name() -> Option<String> {
    let local_ip: IpAddr = match get_local_ipaddr() {
        Some(local_ip) => local_ip,
        None => return None,
    };
    let interfaces = interfaces();
    for iface in interfaces {
        match local_ip {
            IpAddr::V4(local_ipv4) => {
                if iface.ipv4.iter().any(|x| x.addr == local_ipv4) {
                    return Some(iface.name);
                }
            }
            IpAddr::V6(local_ipv6) => {
                if iface.ipv6.iter().any(|x| x.addr == local_ipv6) {
                    return Some(iface.name);
                }
            }
        }
    }
    None
}

/// Get a list of available Network Interfaces
pub fn get_interfaces() -> Vec<Interface> {
    interfaces()
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
