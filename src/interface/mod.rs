pub mod flags;
pub mod interface;
pub mod mtu;
pub mod state;
pub mod types;

use crate::interface::interface::Interface;

#[cfg(feature = "gateway")]
use std::net::IpAddr;

/// Get default Network Interface
#[cfg(feature = "gateway")]
pub fn get_default_interface() -> Result<Interface, String> {
    use crate::net::ip::get_local_ipaddr;

    let ifaces: Vec<Interface> = interfaces();
    for iface in &ifaces {
        if iface.default {
            return Ok(iface.clone());
        }
    }
    let local_ip: IpAddr = match get_local_ipaddr() {
        Some(local_ip) => local_ip,
        None => return Err(String::from("Local IP address not found")),
    };
    let idx: u32 = pick_default_iface_index(&ifaces, local_ip)
        .ok_or_else(|| String::from("Default interface not found"))?;
    ifaces
        .into_iter()
        .find(|it| it.index == idx)
        .ok_or_else(|| String::from("Default interface not found"))
}

/// Get a list of available Network Interfaces
pub fn get_interfaces() -> Vec<Interface> {
    interfaces()
}

/// Pick the interface index corresponding to the system's default route.
/// Prefers exact IP match; falls back to subnet containment.
#[cfg(feature = "gateway")]
pub(crate) fn pick_default_iface_index(ifaces: &[Interface], local_ip: IpAddr) -> Option<u32> {
    let mut subnet_candidate: Option<u32> = None;

    for iface in ifaces {
        match local_ip {
            IpAddr::V4(ipv4) => {
                if iface.ipv4.iter().any(|x| x.addr() == ipv4) {
                    return Some(iface.index);
                }
                if subnet_candidate.is_none() && iface.ipv4.iter().any(|x| x.contains(&ipv4)) {
                    subnet_candidate = Some(iface.index);
                }
            }
            IpAddr::V6(ipv6) => {
                if iface.ipv6.iter().any(|x| x.addr() == ipv6) {
                    return Some(iface.index);
                }
                if subnet_candidate.is_none() && iface.ipv6.iter().any(|x| x.contains(&ipv6)) {
                    subnet_candidate = Some(iface.index);
                }
            }
        }
    }
    subnet_candidate
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
