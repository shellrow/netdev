use core::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Structure of IP Network
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IpNet {
    V4(Ipv4Net),
    V6(Ipv6Net),
}

impl IpNet {
    /// Construct a new IpNet instance from IP Address and Prefix Length
    pub fn new(ip: IpAddr, prefix_len: u8) -> IpNet {
        match ip {
            IpAddr::V4(addr) => Ipv4Net::new(addr, prefix_len).into(),
            IpAddr::V6(addr) => Ipv6Net::new(addr, prefix_len).into(),
        }
    }
    /// Construct a new IpNet instance from IP Address and Network Mask
    pub fn new_with_netmask(ip: IpAddr, netmask: IpAddr) -> IpNet {
        let prefix = ip_netmask_to_prefix(netmask);
        Self::new(ip, prefix)
    }
    /// Returns the address.
    pub fn addr(&self) -> IpAddr {
        match *self {
            IpNet::V4(ref a) => IpAddr::V4(a.addr),
            IpNet::V6(ref a) => IpAddr::V6(a.addr),
        }
    }
    /// Returns the prefix length.
    pub fn prefix_len(&self) -> u8 {
        match *self {
            IpNet::V4(ref a) => a.prefix_len,
            IpNet::V6(ref a) => a.prefix_len,
        }
    }
    /// Returns the maximum valid prefix length.
    pub fn max_prefix_len(&self) -> u8 {
        match *self {
            IpNet::V4(ref a) => a.max_prefix_len(),
            IpNet::V6(ref a) => a.max_prefix_len(),
        }
    }
    /// Returns the network mask.
    pub fn netmask(&self) -> IpAddr {
        match *self {
            IpNet::V4(ref a) => IpAddr::V4(a.netmask()),
            IpNet::V6(ref a) => IpAddr::V6(a.netmask()),
        }
    }
    /// Returns the host mask.
    pub fn hostmask(&self) -> IpAddr {
        match *self {
            IpNet::V4(ref a) => IpAddr::V4(a.hostmask()),
            IpNet::V6(ref a) => IpAddr::V6(a.hostmask()),
        }
    }
    /// Returns the network address.
    pub fn network(&self) -> IpAddr {
        match *self {
            IpNet::V4(ref a) => IpAddr::V4(a.network()),
            IpNet::V6(ref a) => IpAddr::V6(a.network()),
        }
    }
    /// Returns the broadcast address.
    pub fn broadcast(&self) -> IpAddr {
        match *self {
            IpNet::V4(ref a) => IpAddr::V4(a.broadcast()),
            IpNet::V6(ref a) => IpAddr::V6(a.broadcast()),
        }
    }
    /// Checks if the IP Address is in the network.
    pub fn contains(&self, ip: IpAddr) -> bool {
        match *self {
            IpNet::V4(ref a) => match ip {
                IpAddr::V4(ip) => a.contains(ip),
                IpAddr::V6(_) => false,
            },
            IpNet::V6(ref a) => match ip {
                IpAddr::V4(_) => false,
                IpAddr::V6(ip) => a.contains(ip),
            },
        }
    }
}

impl From<Ipv4Net> for IpNet {
    fn from(net: Ipv4Net) -> IpNet {
        IpNet::V4(net)
    }
}

impl From<Ipv6Net> for IpNet {
    fn from(net: Ipv6Net) -> IpNet {
        IpNet::V6(net)
    }
}

impl From<IpAddr> for IpNet {
    fn from(addr: IpAddr) -> IpNet {
        match addr {
            IpAddr::V4(addr) => IpNet::V4(addr.into()),
            IpAddr::V6(addr) => IpNet::V6(addr.into()),
        }
    }
}

/// Structure of IPv4 Network
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Ipv4Net {
    /// IPv4 Address
    pub addr: Ipv4Addr,
    /// Prefix Length
    pub prefix_len: u8,
    /// Network Mask
    pub netmask: Ipv4Addr,
}

impl Ipv4Net {
    /// Construct a new Ipv4Net instance from IPv4 Address and Prefix Length
    pub fn new(ipv4_addr: Ipv4Addr, prefix_len: u8) -> Ipv4Net {
        Ipv4Net {
            addr: ipv4_addr,
            prefix_len: prefix_len,
            netmask: prefix_to_ipv4_netmask(prefix_len),
        }
    }
    /// Construct a new Ipv4Net instance from IPv4 Address and Network Mask
    pub fn new_with_netmask(ipv4_addr: Ipv4Addr, netmask: Ipv4Addr) -> Ipv4Net {
        Ipv4Net {
            addr: ipv4_addr,
            prefix_len: ipv4_netmask_to_prefix(netmask),
            netmask: netmask,
        }
    }
    /// Returns the maximum valid prefix length.
    pub const fn max_prefix_len(&self) -> u8 {
        32
    }
    /// Returns the network mask.
    pub fn netmask(&self) -> Ipv4Addr {
        Ipv4Addr::from(self.netmask_u32())
    }
    /// Returns the network mask. (u32)
    fn netmask_u32(&self) -> u32 {
        u32::max_value()
            .checked_shl(32 - self.prefix_len as u32)
            .unwrap_or(0)
    }
    /// Returns the host mask.
    pub fn hostmask(&self) -> Ipv4Addr {
        Ipv4Addr::from(self.hostmask_u32())
    }
    /// Returns the host mask. (u32)
    fn hostmask_u32(&self) -> u32 {
        u32::max_value()
            .checked_shr(self.prefix_len as u32)
            .unwrap_or(0)
    }
    /// Returns the network address.
    pub fn network(&self) -> Ipv4Addr {
        Ipv4Addr::from(u32::from(self.addr) & self.netmask_u32())
    }
    /// Returns the broadcast address.
    pub fn broadcast(&self) -> Ipv4Addr {
        Ipv4Addr::from(u32::from(self.addr) | self.hostmask_u32())
    }
    /// Checks if the IP Address is in the network.
    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        self.network() == Ipv4Addr::from(u32::from(ip) & self.netmask_u32())
    }
}

impl fmt::Debug for Ipv4Net {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

impl fmt::Display for Ipv4Net {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}/{}", self.addr, self.prefix_len)
    }
}

impl From<Ipv4Addr> for Ipv4Net {
    fn from(addr: Ipv4Addr) -> Ipv4Net {
        Ipv4Net {
            addr,
            prefix_len: 32,
            netmask: prefix_to_ipv4_netmask(32),
        }
    }
}

/// Structure of IPv6 Network
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Ipv6Net {
    /// IPv6 Address
    pub addr: Ipv6Addr,
    /// Prefix Length
    pub prefix_len: u8,
    /// Network Mask
    pub netmask: Ipv6Addr,
}

impl Ipv6Net {
    /// Construct a new Ipv6Net instance from IPv6 Address and Prefix Length
    pub fn new(ipv6_addr: Ipv6Addr, prefix_len: u8) -> Ipv6Net {
        Ipv6Net {
            addr: ipv6_addr,
            prefix_len: prefix_len,
            netmask: prefix_to_ipv6_netmask(prefix_len),
        }
    }
    /// Construct a new Ipv6Net instance from IPv6 Address and Network Mask
    pub fn new_with_netmask(ipv6_addr: Ipv6Addr, netmask: Ipv6Addr) -> Ipv6Net {
        Ipv6Net {
            addr: ipv6_addr,
            prefix_len: ipv6_netmask_to_prefix(netmask),
            netmask: netmask,
        }
    }
    /// Returns the maximum valid prefix length.
    pub const fn max_prefix_len(&self) -> u8 {
        128
    }
    /// Returns the network mask.
    pub fn netmask(&self) -> Ipv6Addr {
        self.netmask_u128().into()
    }
    /// Returns the network mask. (u128)
    fn netmask_u128(&self) -> u128 {
        u128::max_value()
            .checked_shl((128 - self.prefix_len) as u32)
            .unwrap_or(u128::min_value())
    }
    /// Returns the host mask.
    pub fn hostmask(&self) -> Ipv6Addr {
        self.hostmask_u128().into()
    }
    /// Returns the host mask. (u128)
    fn hostmask_u128(&self) -> u128 {
        u128::max_value()
            .checked_shr(self.prefix_len as u32)
            .unwrap_or(u128::min_value())
    }
    /// Returns the network address.
    pub fn network(&self) -> Ipv6Addr {
        (u128::from(self.addr) & self.netmask_u128()).into()
    }
    /// Returns the broadcast address.
    pub fn broadcast(&self) -> Ipv6Addr {
        (u128::from(self.addr) | self.hostmask_u128()).into()
    }
    /// Checks if the IP Address is in the network.
    pub fn contains(&self, ip: Ipv6Addr) -> bool {
        let ipv6_network: Ipv6Addr = (u128::from(ip) & self.netmask_u128()).into();
        self.network() == ipv6_network
    }
}

impl fmt::Debug for Ipv6Net {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

impl fmt::Display for Ipv6Net {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}/{}", self.addr, self.prefix_len)
    }
}

impl From<Ipv6Addr> for Ipv6Net {
    fn from(addr: Ipv6Addr) -> Ipv6Net {
        Ipv6Net {
            addr,
            prefix_len: 128,
            netmask: prefix_to_ipv6_netmask(128),
        }
    }
}

fn ip_netmask_to_prefix(mask: IpAddr) -> u8 {
    match mask {
        IpAddr::V4(mask) => ipv4_netmask_to_prefix(mask),
        IpAddr::V6(mask) => ipv6_netmask_to_prefix(mask),
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
    let netmask_u32: u32 = u32::max_value()
        .checked_shl(32 - prefix_len as u32)
        .unwrap_or(0);
    Ipv4Addr::from(netmask_u32)
}

fn prefix_to_ipv6_netmask(prefix_len: u8) -> Ipv6Addr {
    let netmask_u128: u128 = u128::max_value()
        .checked_shl((128 - prefix_len) as u32)
        .unwrap_or(u128::min_value());
    Ipv6Addr::from(netmask_u128)
}
