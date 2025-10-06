use std::ffi::{CStr, CString};
use std::mem::{self, MaybeUninit};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::raw::c_char;
use std::str::from_utf8_unchecked;

use super::sockaddr::{SockaddrRef, compute_sockaddr_len, netmask_ip_autolen, try_mac_from_raw};
use crate::interface::interface::Interface;
use crate::interface::mtu::get_mtu;
use crate::interface::state::OperState;
use crate::ipnet::{Ipv4Net, Ipv6Net};
use crate::os::unix::types::get_interface_type;
use crate::stats::counters::{InterfaceStats, get_stats};

#[cfg(target_os = "android")]
pub fn unix_interfaces() -> Vec<Interface> {
    use crate::os::android;

    if let Some((getifaddrs, freeifaddrs)) = android::get_libc_ifaddrs() {
        return unix_interfaces_inner(getifaddrs, freeifaddrs);
    }
    Vec::new()
}

#[cfg(not(target_os = "android"))]
pub fn unix_interfaces() -> Vec<Interface> {
    unix_interfaces_inner(libc::getifaddrs, libc::freeifaddrs)
}

fn unix_interfaces_inner(
    getifaddrs: unsafe extern "C" fn(*mut *mut libc::ifaddrs) -> libc::c_int,
    freeifaddrs: unsafe extern "C" fn(*mut libc::ifaddrs),
) -> Vec<Interface> {
    let mut ifaces: Vec<Interface> = vec![];
    let mut addrs: MaybeUninit<*mut libc::ifaddrs> = MaybeUninit::uninit();
    if unsafe { getifaddrs(addrs.as_mut_ptr()) } != 0 {
        return ifaces;
    }
    let addrs = unsafe { addrs.assume_init() };
    let mut addr = addrs;
    while !addr.is_null() {
        let addr_ref: &libc::ifaddrs = unsafe { &*addr };
        let if_type = get_interface_type(addr_ref);
        let c_str = addr_ref.ifa_name as *const c_char;
        let bytes = unsafe { CStr::from_ptr(c_str).to_bytes() };
        let name: String = unsafe { from_utf8_unchecked(bytes).to_owned() };
        let cap: u32 = mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        let addr_len_opt = unsafe { compute_sockaddr_len(addr_ref.ifa_addr, None, Some(cap)) };
        let (mac, ip, ipv6_scope_id) = match addr_len_opt {
            Some(addr_len) => {
                let mac = unsafe {
                    try_mac_from_raw(addr_ref.ifa_addr as *const libc::sockaddr, addr_len)
                };
                let (ip, scope) = unsafe {
                    match SockaddrRef::from_raw(
                        addr_ref.ifa_addr as *const libc::sockaddr,
                        addr_len,
                    ) {
                        Some(sa_ref) => {
                            let ip = sa_ref.to_ip();
                            let scope = sa_ref.to_ipv6_scope();
                            (Some(ip), scope)
                        }
                        None => (None, None),
                    }
                };

                (mac, ip, scope)
            }
            None => (None, None, None),
        };
        let netmask: Option<IpAddr> =
            unsafe { netmask_ip_autolen(addr_ref.ifa_netmask as *const libc::sockaddr) };
        let stats: Option<InterfaceStats> = get_stats(Some(addr_ref), &name);
        let mut ini_ipv4: Option<Ipv4Net> = None;
        let mut ini_ipv6: Option<Ipv6Net> = None;
        if let Some(ip) = ip {
            match ip {
                IpAddr::V4(ipv4) => {
                    let netmask: Ipv4Addr = match netmask {
                        Some(netmask) => match netmask {
                            IpAddr::V4(netmask) => netmask,
                            IpAddr::V6(_) => Ipv4Addr::UNSPECIFIED,
                        },
                        None => Ipv4Addr::UNSPECIFIED,
                    };
                    match Ipv4Net::with_netmask(ipv4, netmask) {
                        Ok(ipv4_net) => ini_ipv4 = Some(ipv4_net),
                        Err(_) => {}
                    }
                }
                IpAddr::V6(ipv6) => {
                    let netmask: Ipv6Addr = match netmask {
                        Some(netmask) => match netmask {
                            IpAddr::V4(_) => Ipv6Addr::UNSPECIFIED,
                            IpAddr::V6(netmask) => netmask,
                        },
                        None => Ipv6Addr::UNSPECIFIED,
                    };
                    match Ipv6Net::with_netmask(ipv6, netmask) {
                        Ok(ipv6_net) => {
                            ini_ipv6 = Some(ipv6_net);
                            if ipv6_scope_id.is_none() {
                                panic!("IPv6 address without scope ID!")
                            }
                        }
                        Err(_) => {}
                    };
                }
            }
        }

        // Check if there is already an interface with this name (since getifaddrs returns one
        // entry per address, so if the interface has multiple addresses, it returns multiple entries).
        // If so, add the IP addresses from the current entry into the existing interface. Otherwise, add a new interface.
        let mut found: bool = false;
        for iface in &mut ifaces {
            if name == iface.name {
                if let Some(mac) = mac.clone() {
                    iface.mac_addr = Some(mac);
                }

                if iface.stats.is_none() {
                    iface.stats = stats.clone();
                }

                if ini_ipv4.is_some() {
                    iface.ipv4.push(ini_ipv4.unwrap());
                }

                if ini_ipv6.is_some() {
                    iface.ipv6.push(ini_ipv6.unwrap());
                    iface.ipv6_scope_ids.push(ipv6_scope_id.unwrap());
                }
                found = true;
            }
        }
        if !found {
            let interface: Interface = Interface {
                index: 0, // We will set these below
                name: name.clone(),
                friendly_name: None,
                description: None,
                if_type: if_type,
                mac_addr: mac.clone(),
                ipv4: match ini_ipv4 {
                    Some(ipv4_addr) => vec![ipv4_addr],
                    None => vec![],
                },
                ipv6: match ini_ipv6 {
                    Some(ipv6_addr) => vec![ipv6_addr],
                    None => vec![],
                },
                ipv6_scope_ids: match ini_ipv6 {
                    Some(_) => vec![ipv6_scope_id.unwrap()],
                    None => vec![],
                },
                flags: addr_ref.ifa_flags,
                oper_state: OperState::from_if_flags(addr_ref.ifa_flags),
                transmit_speed: None,
                receive_speed: None,
                stats: stats,
                #[cfg(feature = "gateway")]
                gateway: None,
                #[cfg(feature = "gateway")]
                dns_servers: Vec::new(),
                mtu: get_mtu(addr_ref, &name),
                #[cfg(feature = "gateway")]
                default: false,
            };
            ifaces.push(interface);
        }
        addr = addr_ref.ifa_next;
    }
    unsafe {
        freeifaddrs(addrs);
    }
    for iface in &mut ifaces {
        let name = CString::new(iface.name.as_bytes()).unwrap();
        unsafe {
            iface.index = libc::if_nametoindex(name.as_ptr());
        }
    }
    ifaces
}
