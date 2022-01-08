use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR};
use windows::Win32::NetworkManagement::IpHelper::{GetAdaptersInfo, IP_ADAPTER_INFO, IP_ADDR_STRING, SendARP};
use std::convert::TryInto;
use std::mem;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ffi::CStr;
use core::ffi::c_void;

use crate::interface::{Interface, MacAddr};
use crate::gateway::Gateway;

pub const EXCEPTION_INTERFACE_INDEX: u32 = 0;

// Convert C string to Rust string without trailing null bytes
fn bytes_to_string(bytes: &[u8]) -> String {
    let result: String = match CStr::from_bytes_with_nul(bytes) {
        Ok(cstr) => {
            match cstr.to_str() {
                Ok(rstr) => rstr.to_string(),
                Err(_) => cstr.to_string_lossy().replace("\u{0}", "").to_string(),
            }  
        },
        Err(_) => {
            String::from_utf8_lossy(bytes).replace("\u{0}", "").to_string()
        }
    };
    result
}

#[cfg(target_endian = "little")]
fn htonl(val : u32) -> u32 {
    let o3 = (val >> 24) as u8;
    let o2 = (val >> 16) as u8;
    let o1 = (val >> 8)  as u8;
    let o0 =  val        as u8;
    (o0 as u32) << 24 | (o1 as u32) << 16 | (o2 as u32) << 8 | (o3 as u32)
}

#[cfg(target_endian = "big")]
fn htonl(val : u32) -> u32 {
    val
}

fn get_mac_through_arp(src_ip: Ipv4Addr, dst_ip: Ipv4Addr) -> MacAddr {
    let src_ip_int: u32 = htonl(u32::from(src_ip));
    let dst_ip_int: u32 = htonl(u32::from(dst_ip));
    let mut out_buf_len : u32 = 6;
    let mut target_mac_addr: [u8; 6] =  [0; 6];
    let res = unsafe { SendARP(dst_ip_int, src_ip_int, target_mac_addr.as_mut_ptr() as *mut c_void, &mut out_buf_len) };
    if res == NO_ERROR {
        MacAddr::new(target_mac_addr)
    }else{
        MacAddr::zero()
    }
}

// Get network interfaces using the IP Helper API
// TODO: Make more rusty ...
// Reference: https://docs.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersinfo
pub fn get_interfaces() -> Vec<Interface> {
    let mut interfaces: Vec<Interface> = vec![];
    let mut out_buf_len : u32 = mem::size_of::<IP_ADAPTER_INFO>().try_into().unwrap();
    let mut raw_adaptor_mem: Vec<u8> = Vec::with_capacity(out_buf_len  as usize);
    let mut p_adaptor: *mut IP_ADAPTER_INFO;
    let mut res = unsafe { GetAdaptersInfo(raw_adaptor_mem.as_mut_ptr() as *mut IP_ADAPTER_INFO, &mut out_buf_len ) };
    // Make an initial call to GetAdaptersInfo to get the necessary size into the out_buf_len variable
    if res == ERROR_BUFFER_OVERFLOW {
		raw_adaptor_mem = Vec::with_capacity(out_buf_len as usize);
		unsafe {
			res = GetAdaptersInfo(raw_adaptor_mem.as_mut_ptr() as *mut IP_ADAPTER_INFO, &mut out_buf_len);
		}
	}
    if res != NO_ERROR {
        return interfaces;
	}
    //Enumerate all adapters
	p_adaptor = unsafe { mem::transmute(&raw_adaptor_mem) };
    while p_adaptor as u64 != 0 {
        unsafe {
			let adapter: IP_ADAPTER_INFO = *p_adaptor;
            if adapter.Index == EXCEPTION_INTERFACE_INDEX{
                p_adaptor = (*p_adaptor).Next;
                continue;
            }
            let adapter_name: String = bytes_to_string(&adapter.AdapterName);
            let adapter_desc: String = bytes_to_string(&adapter.Description);
            let mac_addr:[u8; 6] = adapter.Address[..6].try_into().unwrap_or([0, 0, 0, 0, 0, 0]);
            //Enumerate all IPs
            let mut ipv4_vec: Vec<Ipv4Addr> = vec![];
            let mut ipv6_vec: Vec<Ipv6Addr> = vec![];
            let mut p_ip_addr: *mut IP_ADDR_STRING;
            p_ip_addr = mem::transmute(&(*p_adaptor).IpAddressList);
            while p_ip_addr as u64 != 0 {
                let ip_addr_string: IP_ADDR_STRING = *p_ip_addr;
                let ip_addr: String = bytes_to_string(&ip_addr_string.IpAddress.String);
                match ip_addr.parse::<IpAddr>() {
                    Ok(ip_addr) => {
                        match ip_addr {
                            IpAddr::V4(ipv4_addr) => {
                                ipv4_vec.push(ipv4_addr);
                            },
                            IpAddr::V6(ipv6_addr) => {
                                ipv6_vec.push(ipv6_addr);
                            }
                        }
                    },
                    Err(_) => {},
                }
                p_ip_addr = (*p_ip_addr).Next;
            }
            //Enumerate all gateways
            let mut gateway_ips: Vec<IpAddr> = vec![];
            let mut p_gateway_addr: *mut IP_ADDR_STRING;
            p_gateway_addr = mem::transmute(&(*p_adaptor).GatewayList);
            while p_gateway_addr as u64 != 0 {
                let gateway_addr_string: IP_ADDR_STRING = *p_gateway_addr;
                let gateway_addr: String = bytes_to_string(&gateway_addr_string.IpAddress.String);
                match gateway_addr.parse::<IpAddr>() {
                    Ok(ip_addr) => {
                        gateway_ips.push(ip_addr);
                    },
                    Err(_) => {},
                }
                p_gateway_addr = (*p_gateway_addr).Next;
            }
            let default_gateway: Option<Gateway> = match gateway_ips.get(0) {
                Some(gateway_ip) => {
                    let gateway_ip: IpAddr = *gateway_ip;
                    let default_gateway: Option<Gateway> = if gateway_ip != IpAddr::V4(Ipv4Addr::UNSPECIFIED) {
                        match gateway_ip {
                            IpAddr::V4(dst_ip) => {
                                if let Some(src_ip) = ipv4_vec.get(0) {
                                    let mac_addr = get_mac_through_arp(*src_ip, dst_ip);
                                    let gateway = Gateway {
                                        mac_addr: mac_addr,
                                        ip_addr: IpAddr::V4(dst_ip),
                                    };
                                    Some(gateway)
                                }else{
                                    None
                                }
                            },
                            IpAddr::V6(_dst_ip) => {
                                None
                            },
                        }
                    }else{
                        None
                    };
                    default_gateway
                },
                None => None,
            };
            let interface: Interface = Interface{
                index: adapter.Index,
                name: adapter_name,
                description: Some(adapter_desc),
                mac_addr: Some(MacAddr::new(mac_addr)),
                ipv4: ipv4_vec,
                ipv6: ipv6_vec,
                gateway: default_gateway,
            };
            interfaces.push(interface);
		}
        unsafe { p_adaptor = (*p_adaptor).Next; }
    }
    return interfaces;
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use super::*;
    #[test]
    fn test_nw_interfaces() {
        let interfaces = get_interfaces();
        for interface in interfaces {
            println!("{:?}", interface);
        }
    }
    #[test]
    fn test_arp() {
        let src_ip: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 2);
        let dst_ip: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 1);
        let mac_addr = get_mac_through_arp(src_ip, dst_ip);
        println!("{}", mac_addr);
    }
}
