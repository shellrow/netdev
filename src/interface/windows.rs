use core::ffi::c_void;
use libc::{c_char, strlen, wchar_t, wcslen};
use memalloc::{allocate, deallocate};
use std::convert::TryFrom;
use std::convert::TryInto;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, SendARP, GAA_FLAG_INCLUDE_GATEWAYS, IP_ADAPTER_ADDRESSES_LH,
};
use windows_sys::Win32::NetworkManagement::Ndis::{IF_OPER_STATUS, NET_IF_OPER_STATUS_UP};
use windows_sys::Win32::Networking::WinSock::SOCKET_ADDRESS;
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6,
};

use crate::device::NetworkDevice;
use crate::interface::{Interface, InterfaceType};
use crate::ip::{Ipv4Net, Ipv6Net};
use crate::mac::MacAddr;
use crate::sys;

#[cfg(target_endian = "little")]
fn htonl(val: u32) -> u32 {
    let o3 = (val >> 24) as u8;
    let o2 = (val >> 16) as u8;
    let o1 = (val >> 8) as u8;
    let o0 = val as u8;
    (o0 as u32) << 24 | (o1 as u32) << 16 | (o2 as u32) << 8 | (o3 as u32)
}

#[cfg(target_endian = "big")]
fn htonl(val: u32) -> u32 {
    val
}

fn get_mac_through_arp(src_ip: Ipv4Addr, dst_ip: Ipv4Addr) -> MacAddr {
    let src_ip_int: u32 = htonl(u32::from(src_ip));
    let dst_ip_int: u32 = htonl(u32::from(dst_ip));
    let mut out_buf_len: u32 = 6;
    let mut target_mac_addr: [u8; 6] = [0; 6];
    let res = unsafe {
        SendARP(
            dst_ip_int,
            src_ip_int,
            target_mac_addr.as_mut_ptr() as *mut c_void,
            &mut out_buf_len,
        )
    };
    if res == NO_ERROR {
        MacAddr::from_octets(target_mac_addr)
    } else {
        MacAddr::zero()
    }
}

unsafe fn socket_address_to_ipaddr(addr: &SOCKET_ADDRESS) -> Option<IpAddr> {
    let sockaddr = unsafe { *addr.lpSockaddr };
    if sockaddr.sa_family == AF_INET {
        let sockaddr: *mut SOCKADDR_IN = addr.lpSockaddr as *mut SOCKADDR_IN;
        let a = unsafe { (*sockaddr).sin_addr.S_un.S_addr };
        let ipv4 = if cfg!(target_endian = "little") {
            Ipv4Addr::from(a.swap_bytes())
        } else {
            Ipv4Addr::from(a)
        };
        return Some(IpAddr::V4(ipv4));
    } else if sockaddr.sa_family == AF_INET6 {
        let sockaddr: *mut SOCKADDR_IN6 = addr.lpSockaddr as *mut SOCKADDR_IN6;
        let a = unsafe { (*sockaddr).sin6_addr.u.Byte };
        let ipv6 = Ipv6Addr::from(a);
        return Some(IpAddr::V6(ipv6));
    }
    None
}

pub fn is_running(interface: &Interface) -> bool {
    interface.is_up()
}

pub fn is_physical_interface(interface: &Interface) -> bool {
    interface.is_up() && interface.is_running() && !interface.is_tun() && !interface.is_loopback()
}

// Get network interfaces using the IP Helper API
// Reference: https://docs.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses
pub fn interfaces() -> Vec<Interface> {
    let local_ip: IpAddr = match super::get_local_ipaddr() {
        Some(local_ip) => local_ip,
        None => IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    let mut interfaces: Vec<Interface> = vec![];
    let mut dwsize: u32 = 2000;
    let mut mem = unsafe { allocate(dwsize as usize) } as *mut IP_ADAPTER_ADDRESSES_LH;
    let mut retries = 3;
    let mut ret_val;
    loop {
        let old_size = dwsize as usize;
        ret_val = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_INCLUDE_GATEWAYS,
                std::ptr::null_mut::<std::ffi::c_void>(),
                mem,
                &mut dwsize,
            )
        };
        if ret_val != ERROR_BUFFER_OVERFLOW || retries <= 0 {
            break;
        }
        unsafe { deallocate(mem as *mut u8, old_size as usize) };
        mem = unsafe { allocate(dwsize as usize) as *mut IP_ADAPTER_ADDRESSES_LH };
        retries -= 1;
    }
    if ret_val == NO_ERROR {
        // Enumerate all adapters
        let mut cur = mem;
        while !cur.is_null() {
            let if_type_int: u32 = unsafe { (*cur).IfType };
            let if_type = match InterfaceType::try_from(if_type_int) {
                Ok(if_type) => if_type,
                Err(_) => {
                    cur = unsafe { (*cur).Next };
                    continue;
                }
            };
            // Index
            let anon1 = unsafe { (*cur).Anonymous1 };
            let anon = unsafe { anon1.Anonymous };
            let index = anon.IfIndex;
            // Flags and Status
            let mut flags: u32 = 0;
            let status: IF_OPER_STATUS = unsafe { (*cur).OperStatus };
            if status == NET_IF_OPER_STATUS_UP {
                flags |= sys::IFF_UP;
            }
            match if_type {
                InterfaceType::Ethernet
                | InterfaceType::TokenRing
                | InterfaceType::Wireless80211
                | InterfaceType::HighPerformanceSerialBus => {
                    flags |= sys::IFF_BROADCAST | sys::IFF_MULTICAST;
                }
                InterfaceType::Ppp | InterfaceType::Tunnel => {
                    flags |= sys::IFF_POINTOPOINT | sys::IFF_MULTICAST;
                }
                InterfaceType::Loopback => {
                    flags |= sys::IFF_LOOPBACK | sys::IFF_MULTICAST;
                }
                InterfaceType::Atm => {
                    flags |= sys::IFF_BROADCAST | sys::IFF_POINTOPOINT | sys::IFF_MULTICAST;
                }
                _ => {}
            }
            // Name
            let p_aname = unsafe { (*cur).AdapterName };
            let aname_len = unsafe { strlen(p_aname as *const c_char) };
            let aname_slice = unsafe { std::slice::from_raw_parts(p_aname, aname_len) };
            let adapter_name = String::from_utf8(aname_slice.to_vec()).unwrap();
            // Friendly Name
            let p_fname = unsafe { (*cur).FriendlyName };
            let fname_len = unsafe { wcslen(p_fname as *const wchar_t) };
            let fname_slice = unsafe { std::slice::from_raw_parts(p_fname, fname_len) };
            let friendly_name = String::from_utf16(fname_slice).unwrap();
            // Description
            let p_desc = unsafe { (*cur).Description };
            let desc_len = unsafe { wcslen(p_desc as *const wchar_t) };
            let desc_slice = unsafe { std::slice::from_raw_parts(p_desc, desc_len) };
            let description = String::from_utf16(desc_slice).unwrap();
            // MAC address
            let mac_addr_arr: [u8; 6] = unsafe { (*cur).PhysicalAddress }[..6]
                .try_into()
                .unwrap_or([0, 0, 0, 0, 0, 0]);
            let mac_addr: MacAddr = MacAddr::from_octets(mac_addr_arr);
            // TransmitLinkSpeed (bits per second)
            let transmit_speed = unsafe { (*cur).TransmitLinkSpeed };
            // ReceiveLinkSpeed (bits per second)
            let receive_speed = unsafe { (*cur).ReceiveLinkSpeed };
            let mut ipv4_vec: Vec<Ipv4Net> = vec![];
            let mut ipv6_vec: Vec<Ipv6Net> = vec![];
            // Enumerate all IPs
            let mut cur_a = unsafe { (*cur).FirstUnicastAddress };
            while !cur_a.is_null() {
                let addr: SOCKET_ADDRESS = unsafe { (*cur_a).Address };
                let ip_addr = unsafe { socket_address_to_ipaddr(&addr) };
                let prefix_len = unsafe { (*cur_a).OnLinkPrefixLength };
                if let Some(ip_addr) = ip_addr {
                    match ip_addr {
                        IpAddr::V4(ipv4) => {
                            let ipv4_net: Ipv4Net = Ipv4Net::new(ipv4, prefix_len);
                            ipv4_vec.push(ipv4_net);
                        }
                        IpAddr::V6(ipv6) => {
                            let ipv6_net: Ipv6Net = Ipv6Net::new(ipv6, prefix_len);
                            ipv6_vec.push(ipv6_net);
                        }
                    }
                }
                cur_a = unsafe { (*cur_a).Next };
            }
            // Gateway
            let mut gateway_ips: Vec<IpAddr> = vec![];
            let mut cur_g = unsafe { (*cur).FirstGatewayAddress };
            while !cur_g.is_null() {
                let addr: SOCKET_ADDRESS = unsafe { (*cur_g).Address };
                if let Some(ip_addr) = unsafe { socket_address_to_ipaddr(&addr) } {
                    gateway_ips.push(ip_addr);
                }
                cur_g = unsafe { (*cur_g).Next };
            }
            let mut default_gateway: NetworkDevice = NetworkDevice::new();
            if flags & sys::IFF_UP != 0 {
                for gateway_ip in gateway_ips {
                    match gateway_ip {
                        IpAddr::V4(ipv4) => {
                            if let Some(ip_net) = ipv4_vec.get(0) {
                                let mac_addr = get_mac_through_arp(ip_net.addr, ipv4);
                                default_gateway.mac_addr = mac_addr;
                                default_gateway.ipv4.push(ipv4);
                            }
                        }
                        IpAddr::V6(ipv6) => {
                            if let Some(_ip_net) = ipv6_vec.get(0) {
                                default_gateway.ipv6.push(ipv6);
                            }
                        }
                    }
                }
            }
            // DNS Servers
            let mut dns_servers: Vec<IpAddr> = vec![];
            let mut cur_d = unsafe { (*cur).FirstDnsServerAddress };
            while !cur_d.is_null() {
                let addr: SOCKET_ADDRESS = unsafe { (*cur_d).Address };
                if let Some(ip_addr) = unsafe { socket_address_to_ipaddr(&addr) } {
                    dns_servers.push(ip_addr);
                }
                cur_d = unsafe { (*cur_d).Next };
            }
            let default: bool = match local_ip {
                IpAddr::V4(local_ipv4) => ipv4_vec.iter().any(|x| x.addr == local_ipv4),
                IpAddr::V6(local_ipv6) => ipv6_vec.iter().any(|x| x.addr == local_ipv6),
            };
            let interface: Interface = Interface {
                index: index,
                name: adapter_name,
                friendly_name: Some(friendly_name),
                description: Some(description),
                if_type: if_type,
                mac_addr: Some(mac_addr),
                ipv4: ipv4_vec,
                ipv6: ipv6_vec,
                flags: flags,
                transmit_speed: Some(transmit_speed),
                receive_speed: Some(receive_speed),
                gateway: if default_gateway.mac_addr == MacAddr::zero() {
                    None
                } else {
                    Some(default_gateway)
                },
                dns_servers: dns_servers,
                default: default,
            };
            interfaces.push(interface);
            cur = unsafe { (*cur).Next };
        }
    } else {
        unsafe {
            deallocate(mem as *mut u8, dwsize as usize);
        }
    }
    unsafe {
        deallocate(mem as *mut u8, dwsize as usize);
    }
    return interfaces;
}
