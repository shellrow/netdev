use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Clone, Debug)]
pub struct Ipv4Net {
    pub addr: Ipv4Addr,
    pub prefix_len: u8,
    pub netmask: Ipv4Addr,
}

impl Ipv4Net {
    pub fn new(ipv4_addr: Ipv4Addr, prefix_len: u8) -> Ipv4Net {
        Ipv4Net {
            addr: ipv4_addr,
            prefix_len: prefix_len,
            netmask: prefix_to_ipv4_netmask(prefix_len),
        }
    }
    pub fn new_with_netmask(ipv4_addr: Ipv4Addr, netmask: Ipv4Addr) -> Ipv4Net {
        Ipv4Net {
            addr: ipv4_addr,
            prefix_len: ipv4_netmask_to_prefix(netmask),
            netmask: netmask,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ipv6Net {
    pub addr: Ipv6Addr,
    pub prefix_len: u8,
    pub netmask: Ipv6Addr,
}

impl Ipv6Net {
    pub fn new(ipv6_addr: Ipv6Addr, prefix_len: u8) -> Ipv6Net {
        Ipv6Net {
            addr: ipv6_addr,
            prefix_len: prefix_len,
            netmask: prefix_to_ipv6_netmask(prefix_len),
        }
    }
    pub fn new_with_netmask(ipv6_addr: Ipv6Addr, netmask: Ipv6Addr) -> Ipv6Net {
        Ipv6Net {
            addr: ipv6_addr,
            prefix_len: ipv6_netmask_to_prefix(netmask),
            netmask: netmask,
        }
    }
}

fn ipv4_netmask_to_prefix(netmask: Ipv4Addr) -> u8 {
    let netmask = u32::from(netmask);
    let prefix = (!netmask).leading_zeros() as u8;
    if (u64::from(netmask) << prefix) & 0xffff_ffff != 0 {
        0
    } else {
        prefix
    }
}

fn ipv6_netmask_to_prefix(netmask: Ipv6Addr) -> u8 {
    let netmask = netmask.segments();
    let mut mask_iter = netmask.iter();
    let mut prefix = 0;
    for &segment in &mut mask_iter {
        if segment == 0xffff {
            prefix += 16;
        } else if segment == 0 {
            break;
        } else {
            let prefix_bits = (!segment).leading_zeros() as u8;
            if segment << prefix_bits != 0 {
                return 0;
            }
            prefix += prefix_bits;
            break;
        }
    }
    for &segment in mask_iter {
        if segment != 0 {
            return 0;
        }
    }
    prefix
}

fn prefix_to_ipv4_netmask(prefix_len: u8) -> Ipv4Addr {
    let netmask_u32: u32 = u32::max_value().checked_shl(32 - prefix_len as u32).unwrap_or(0);
    Ipv4Addr::from(netmask_u32)
}

fn prefix_to_ipv6_netmask(prefix_len: u8) -> Ipv6Addr {
    let netmask_u128: u128 = u128::max_value().checked_shl((128 - prefix_len) as u32).unwrap_or(u128::min_value());
    Ipv6Addr::from(netmask_u128)
}
