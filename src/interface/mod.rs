pub mod flags;
pub mod interface;
pub mod mtu;
pub mod state;
pub mod types;

use crate::interface::interface::Interface;

/// Get default Network Interface
#[cfg(feature = "gateway")]
pub fn get_default_interface() -> Result<Interface, String> {
    use crate::net::ip::get_local_ipaddr;
    use std::net::IpAddr;

    let interfaces: Vec<Interface> = interfaces();
    for iface in &interfaces {
        if iface.default {
            return Ok(iface.clone());
        }
    }
    let local_ip: IpAddr = match get_local_ipaddr() {
        Some(local_ip) => local_ip,
        None => return Err(String::from("Local IP address not found")),
    };
    for iface in interfaces {
        match local_ip {
            IpAddr::V4(local_ipv4) => {
                if iface.ipv4.iter().any(|x| x.addr() == local_ipv4) {
                    return Ok(iface);
                }
            }
            IpAddr::V6(local_ipv6) => {
                if iface.ipv6.iter().any(|x| x.addr() == local_ipv6) {
                    return Ok(iface);
                }
            }
        }
    }
    Err(String::from("Default Interface not found"))
}

/// Get a list of available Network Interfaces
pub fn get_interfaces() -> Vec<Interface> {
    interfaces()
}

pub(crate) fn interfaces() -> Vec<Interface> {
    #[cfg(target_os = "linux")]
    {
        crate::os::linux::interface::interfaces()
    }
    #[cfg(target_os = "android")]
    {
        crate::os::android::interface::interfaces()
    }
    #[cfg(target_os = "windows")]
    {
        crate::os::windows::interface::interfaces()
    }
    #[cfg(target_os = "macos")]
    {
        crate::os::macos::interface::interfaces()
    }
    //#[cfg(target_os = "ios")]
    #[cfg(all(target_vendor = "apple", not(target_os = "macos")))]
    {
        crate::os::ios::interface::interfaces()
    }
    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    {
        crate::os::bsd::interface::interfaces()
    }
}
