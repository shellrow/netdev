#![allow(non_camel_case_types)]

#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "netbsd")]
mod netbsd;
#[cfg(target_os = "openbsd")]
mod openbsd;

use libc::{c_int, size_t};
use std::{
    collections::HashMap,
    ffi::c_void,
    io, mem,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
};

#[cfg(target_os = "freebsd")]
use self::freebsd::{SOCKADDR_ALIGN, message_header_len, rt_msghdr};
#[cfg(target_os = "netbsd")]
use self::netbsd::{SOCKADDR_ALIGN, message_header_len, rt_msghdr};
#[cfg(target_os = "openbsd")]
use self::openbsd::{SOCKADDR_ALIGN, message_header_len, rt_msghdr};

use crate::net::{device::NetworkDevice, mac::MacAddr};

const CTL_NET: c_int = libc::CTL_NET;
const NET_RT_DUMP: c_int = 1;
const NET_RT_FLAGS: c_int = 2;

#[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
const RTF_LLINFO: u32 = 1024;
#[cfg(target_os = "netbsd")]
const RTF_LLDATA: u32 = 1024;
#[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
const RTM_VERSION: u8 = 5;
#[cfg(target_os = "netbsd")]
const RTM_VERSION: u8 = 4;

const RTAX_DST: usize = 0;
const RTAX_GATEWAY: usize = 1;
const RTAX_NETMASK: usize = 2;

#[cfg(target_os = "freebsd")]
const RTAX_MAX: usize = 8;
#[cfg(target_os = "netbsd")]
const RTAX_MAX: usize = 9;
#[cfg(target_os = "openbsd")]
const RTAX_MAX: usize = 15;

unsafe extern "C" {
    fn sysctl(
        name: *mut c_int,
        namelen: u32,
        oldp: *mut c_void,
        oldlenp: *mut size_t,
        newp: *mut c_void,
        newlen: size_t,
    ) -> c_int;
}

/// Fetches a sysctl value into a Vec<u8>.
fn sysctl_vec(mib: &mut [c_int]) -> io::Result<Vec<u8>> {
    let mut len: size_t = 0;
    let mut r = unsafe {
        sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            ptr::null_mut(),
            &mut len,
            ptr::null_mut(),
            0,
        )
    };
    if r < 0 {
        return Err(io::Error::last_os_error());
    }

    let mut buf = vec![0u8; len as usize];
    r = unsafe {
        sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            buf.as_mut_ptr() as *mut c_void,
            &mut len,
            ptr::null_mut(),
            0,
        )
    };
    if r < 0 {
        // If the value grew, kernel returns ENOMEM. Retry once.
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ENOMEM) {
            let mut len2: size_t = 0;
            let r2 = unsafe {
                sysctl(
                    mib.as_mut_ptr(),
                    mib.len() as u32,
                    ptr::null_mut(),
                    &mut len2,
                    ptr::null_mut(),
                    0,
                )
            };
            if r2 < 0 {
                return Err(io::Error::last_os_error());
            }
            buf.resize(len2 as usize, 0);
            let r3 = unsafe {
                sysctl(
                    mib.as_mut_ptr(),
                    mib.len() as u32,
                    buf.as_mut_ptr() as *mut c_void,
                    &mut len2,
                    ptr::null_mut(),
                    0,
                )
            };
            if r3 < 0 {
                return Err(io::Error::last_os_error());
            }
            buf.truncate(len2 as usize);
            return Ok(buf);
        }
        return Err(err);
    }
    buf.truncate(len as usize);
    Ok(buf)
}

#[inline]
fn roundup(len: usize) -> usize {
    if len == 0 {
        SOCKADDR_ALIGN
    } else {
        (len + (SOCKADDR_ALIGN - 1)) & !(SOCKADDR_ALIGN - 1)
    }
}

#[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
#[inline]
fn has_embedded_scope(addr: Ipv6Addr) -> bool {
    let oct = addr.octets();
    // Unicast link-local: fe80::/10.
    let is_unicast_ll = oct[0] == 0xfe && (oct[1] & 0xc0) == 0x80;

    // Multicast local scopes: 0x1 (interface-local) or 0x2 (link-local).
    let is_multicast = oct[0] == 0xff;
    let mscope = oct[1] & 0x0f;
    let is_local_scope_mc = is_multicast && (mscope == 0x1 || mscope == 0x2);

    is_unicast_ll || is_local_scope_mc
}

#[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
#[inline]
fn normalize_scoped_v6(addr: Ipv6Addr, scope_id: u32) -> (Ipv6Addr, u32) {
    if scope_id != 0 || !has_embedded_scope(addr) {
        return (addr, scope_id);
    }

    let mut octets = addr.octets();
    let embedded_scope = u16::from_be_bytes([octets[2], octets[3]]) as u32;
    octets[2] = 0;
    octets[3] = 0;
    (Ipv6Addr::from(octets), embedded_scope)
}

#[cfg(target_os = "freebsd")]
#[inline]
fn normalize_scoped_v6(addr: Ipv6Addr, scope_id: u32) -> (Ipv6Addr, u32) {
    // FreeBSD exposes the scope separately (or leaves it for sockaddr_dl), unlike the
    // KAME representation still used by NetBSD and OpenBSD routing messages.
    (addr, scope_id)
}

#[inline]
fn normalize_gateway(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(v4) => IpAddr::V4(v4),
        IpAddr::V6(v6) => IpAddr::V6(normalize_scoped_v6(v6, 0).0),
    }
}

/// Parse a bounded routing-socket address without requiring aligned storage.
fn ip_from_sockaddr(sa: &[u8]) -> Option<IpAddr> {
    match i32::from(*sa.get(1)?) {
        libc::AF_INET if sa.len() >= mem::size_of::<libc::sockaddr_in>() => Some(IpAddr::V4(
            Ipv4Addr::from(<[u8; 4]>::try_from(sa.get(4..8)?).ok()?),
        )),
        libc::AF_INET6 if sa.len() >= mem::size_of::<libc::sockaddr_in6>() => Some(IpAddr::V6(
            Ipv6Addr::from(<[u8; 16]>::try_from(sa.get(8..24)?).ok()?),
        )),
        _ => None,
    }
}

fn masklen_from_sockaddr(dst: IpAddr, mask: &[u8]) -> u8 {
    // Routing sockets omit trailing zero bytes from masks, including all bytes
    // except the length byte for a default-route mask.
    match dst {
        IpAddr::V4(_) => {
            let mut bytes = [0u8; 4];
            for (out, value) in bytes.iter_mut().zip(mask.get(4..).unwrap_or_default()) {
                *out = *value;
            }
            u32::from_be_bytes(bytes).leading_ones() as u8
        }
        IpAddr::V6(_) => {
            let mut bytes = [0u8; 16];
            for (out, value) in bytes.iter_mut().zip(mask.get(8..).unwrap_or_default()) {
                *out = *value;
            }
            u128::from_be_bytes(bytes).leading_ones() as u8
        }
    }
}

fn code_to_error(err: i32) -> io::Error {
    let kind = match err {
        x if x == libc::EEXIST => io::ErrorKind::AlreadyExists,
        x if x == libc::ESRCH => io::ErrorKind::NotFound,
        x if x == libc::ENOBUFS => io::ErrorKind::OutOfMemory,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, format!("rtm_errno {}", err))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct NeighborEntry {
    ip: IpAddr,
    ifindex: u32,
    mac: MacAddr,
}

const SOCKADDR_DL_HEADER_LEN: usize = 8;

/// Extract a neighbor entry from a routing message's address block.
fn message_to_neighbor(msg: &[u8], header_ifindex: u32) -> Option<NeighborEntry> {
    let mut off = 0usize;
    let mut ip: Option<IpAddr> = None;
    let mut scope_id = 0u32;
    let mut link_ifindex = 0u32;
    let mut mac: Option<MacAddr> = None;

    let make_entry =
        |ip: Option<IpAddr>, mac: Option<MacAddr>, link_ifindex: u32, scope_id: u32| {
            Some(NeighborEntry {
                ip: ip?,
                ifindex: if link_ifindex != 0 {
                    link_ifindex
                } else if scope_id != 0 {
                    scope_id
                } else {
                    header_ifindex
                },
                mac: mac?,
            })
        };

    while off + 2 <= msg.len() {
        let sa_len = msg[off] as usize;

        if sa_len == 0 {
            off += roundup(0);
            continue;
        }
        if off + sa_len > msg.len() {
            off += roundup(sa_len);
            continue;
        }

        let sa = &msg[off..off + sa_len];
        if sa.len() < 2 {
            return None;
        }
        match sa[1] as c_int {
            x if x == libc::AF_INET => {
                ip = ip_from_sockaddr(sa);
                if let Some(entry) = make_entry(ip, mac, link_ifindex, scope_id) {
                    return Some(entry);
                }
            }
            x if x == libc::AF_INET6 => {
                if let Some(IpAddr::V6(v6)) = ip_from_sockaddr(sa) {
                    let scope = u32::from_ne_bytes(sa.get(24..28)?.try_into().ok()?);
                    let (v6, normalized_scope) = normalize_scoped_v6(v6, scope);
                    scope_id = normalized_scope;
                    ip = Some(IpAddr::V6(v6));
                    if let Some(entry) = make_entry(ip, mac, link_ifindex, scope_id) {
                        return Some(entry);
                    }
                }
            }
            x if x == libc::AF_LINK => {
                if sa_len < SOCKADDR_DL_HEADER_LEN {
                    off += roundup(sa_len);
                    continue;
                }
                // Only the fixed eight-byte header is required. FreeBSD's libc exposes a
                // larger sockaddr_dl backing array than a routing message is required to carry.
                let sdl = &msg[off..off + sa_len];
                let nlen = sdl[5] as usize;
                let alen = sdl[6] as usize;
                link_ifindex = u16::from_ne_bytes([sdl[2], sdl[3]]) as u32;

                let mac_start = SOCKADDR_DL_HEADER_LEN + nlen;
                if alen >= 6 && mac_start + alen <= sdl.len() {
                    let m = MacAddr::from_octets([
                        sdl[mac_start],
                        sdl[mac_start + 1],
                        sdl[mac_start + 2],
                        sdl[mac_start + 3],
                        sdl[mac_start + 4],
                        sdl[mac_start + 5],
                    ]);
                    mac = Some(m);
                    if let Some(entry) = make_entry(ip, mac, link_ifindex, scope_id) {
                        return Some(entry);
                    }
                }
            }
            _ => {}
        }

        off += roundup(sa_len);
    }

    None
}

#[derive(Debug)]
struct RawRoute {
    #[allow(dead_code)]
    dst: IpAddr,
    prefix: u8,
    gateway: Option<IpAddr>,
    ifindex: u32,
    #[allow(dead_code)]
    flags: c_int,
}

fn parse_one_route(hdr: &rt_msghdr, addr_block: &[u8]) -> Option<RawRoute> {
    let mut addrs: [Option<&[u8]>; RTAX_MAX] = [None; RTAX_MAX];
    let mut off = 0usize;
    for (idx, entry) in addrs.iter_mut().enumerate() {
        if (hdr.rtm_addrs & (1 << idx)) != 0 {
            let len = usize::from(*addr_block.get(off)?);
            let step = roundup(len);
            let bytes = addr_block.get(off..off + step)?;
            *entry = Some(&bytes[..len]);
            off += step;
        }
    }

    let dst_ip = ip_from_sockaddr(addrs[RTAX_DST]?)?;
    let prefix = if let Some(mask) = addrs[RTAX_NETMASK] {
        masklen_from_sockaddr(dst_ip, mask)
    } else {
        match dst_ip {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        }
    };
    let gateway = addrs[RTAX_GATEWAY]
        .and_then(ip_from_sockaddr)
        .map(normalize_gateway);

    Some(RawRoute {
        dst: dst_ip,
        prefix,
        gateway,
        ifindex: hdr.rtm_index as u32,
        flags: hdr.rtm_flags,
    })
}

fn neighbor_mib(address_family: c_int) -> Vec<c_int> {
    let neighbor_flag = {
        #[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
        {
            RTF_LLINFO
        }
        #[cfg(target_os = "netbsd")]
        {
            RTF_LLDATA
        }
    };
    let mib = vec![
        CTL_NET,
        libc::PF_ROUTE,
        0,
        address_family,
        NET_RT_FLAGS,
        neighbor_flag as c_int,
    ];
    #[cfg(target_os = "openbsd")]
    {
        let mut mib = mib;
        mib.push(0); // Default routing domain.
        mib
    }
    #[cfg(not(target_os = "openbsd"))]
    {
        mib
    }
}

/// Read ARP or NDP entries from the BSD routing table via `sysctl`.
fn get_neighbor_entries(address_family: c_int) -> io::Result<Vec<NeighborEntry>> {
    let mut entries = Vec::new();
    let mut mib = neighbor_mib(address_family);
    let buf = sysctl_vec(&mut mib)?;

    let mut off = 0usize;
    while off + mem::size_of::<rt_msghdr>() <= buf.len() {
        let hdr = unsafe { std::ptr::read_unaligned(buf[off..].as_ptr().cast::<rt_msghdr>()) };
        let msglen = hdr.rtm_msglen as usize;
        if msglen == 0 || off + msglen > buf.len() {
            break;
        }

        if hdr.rtm_version != RTM_VERSION {
            off += msglen;
            continue;
        }
        if hdr.rtm_errno != 0 {
            return Err(code_to_error(hdr.rtm_errno));
        }

        let hdrlen = message_header_len(&hdr);
        if hdrlen < mem::size_of::<rt_msghdr>() || off + hdrlen > off + msglen {
            break;
        }
        let addr_block = &buf[off + hdrlen..off + msglen];
        if let Some(entry) = message_to_neighbor(addr_block, hdr.rtm_index as u32) {
            entries.push(entry);
        }

        off += msglen;
    }

    Ok(entries)
}

#[derive(Default)]
struct NeighborTables {
    ipv4: HashMap<(u32, Ipv4Addr), MacAddr>,
    ipv6: HashMap<(u32, Ipv6Addr), MacAddr>,
}

fn get_neighbor_tables() -> NeighborTables {
    let mut tables = NeighborTables::default();

    // Fetch each family independently so failure of one table does not discard the other.
    for entry in get_neighbor_entries(libc::AF_INET).unwrap_or_default() {
        if let IpAddr::V4(ip) = entry.ip {
            tables.ipv4.insert((entry.ifindex, ip), entry.mac);
        }
    }
    for entry in get_neighbor_entries(libc::AF_INET6).unwrap_or_default() {
        if let IpAddr::V6(ip) = entry.ip {
            tables.ipv6.insert((entry.ifindex, ip), entry.mac);
        }
    }

    tables
}

fn route_dump_mib() -> Vec<c_int> {
    let mib = vec![
        CTL_NET,
        libc::PF_ROUTE,
        0,
        0,           // All address families.
        NET_RT_DUMP, // Dump routes.
        0,           // Flags, or route priority on OpenBSD.
    ];
    #[cfg(target_os = "openbsd")]
    {
        let mut mib = mib;
        mib.push(0); // Default routing table.
        mib
    }
    #[cfg(not(target_os = "openbsd"))]
    {
        mib
    }
}

/// Dump the routing table via `sysctl` on BSD and parse each `rt_msghdr`.
fn list_routes() -> io::Result<Vec<RawRoute>> {
    let mut mib = route_dump_mib();
    let buf = sysctl_vec(&mut mib)?;

    let mut out = Vec::<RawRoute>::new();
    let mut off = 0usize;

    while off + mem::size_of::<rt_msghdr>() <= buf.len() {
        let hdr = unsafe { std::ptr::read_unaligned(buf[off..].as_ptr().cast::<rt_msghdr>()) };
        let msglen = hdr.rtm_msglen as usize;
        if msglen == 0 || off + msglen > buf.len() {
            break;
        }

        if hdr.rtm_version != RTM_VERSION {
            off += msglen;
            continue;
        }
        if hdr.rtm_errno != 0 {
            return Err(code_to_error(hdr.rtm_errno));
        }

        let hdrlen = message_header_len(&hdr);
        if hdrlen < mem::size_of::<rt_msghdr>() || off + hdrlen > off + msglen {
            break;
        }
        let addr_block = &buf[off + hdrlen..off + msglen];
        if let Some(rr) = parse_one_route(&hdr, addr_block) {
            out.push(rr);
        }

        off += msglen;
    }

    Ok(out)
}

/// Build a map `ifindex -> NetworkDevice` for default gateways on BSD.
pub fn get_gateway_map() -> HashMap<u32, NetworkDevice> {
    // Fetch routes; on failure just return an empty map.
    let routes = match list_routes() {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };
    let neighbor_tables = get_neighbor_tables();

    build_gateway_map(routes, &neighbor_tables)
}

fn build_gateway_map(
    routes: Vec<RawRoute>,
    neighbor_tables: &NeighborTables,
) -> HashMap<u32, NetworkDevice> {
    // Accumulator: ifindex -> (optional MAC candidate, v4 list, v6 list)
    #[derive(Default)]
    struct Acc {
        ipv4_mac: Option<MacAddr>,
        ipv6_mac: Option<MacAddr>,
        v4: Vec<Ipv4Addr>,
        v6: Vec<Ipv6Addr>,
    }
    let mut acc: HashMap<u32, Acc> = HashMap::new();

    // Small helpers to avoid duplicates
    fn push_v4(vec: &mut Vec<Ipv4Addr>, ip: Ipv4Addr) {
        if !vec.iter().any(|x| *x == ip) {
            vec.push(ip);
        }
    }
    fn push_v6(vec: &mut Vec<Ipv6Addr>, ip: Ipv6Addr) {
        if !vec.iter().any(|x| *x == ip) {
            vec.push(ip);
        }
    }

    // Collect gateway candidates per outgoing interface
    for r in routes.into_iter() {
        // Only gateway routes (prefix==0) and those flagged as GATEWAY
        if r.prefix != 0 || (r.flags & libc::RTF_GATEWAY) == 0 {
            continue;
        }
        let gw = match r.gateway {
            Some(ip) => ip,
            None => continue,
        };

        let entry = acc.entry(r.ifindex).or_default();

        match gw {
            IpAddr::V4(v4) => {
                if let Some(mac) = neighbor_tables.ipv4.get(&(r.ifindex, v4)).copied() {
                    entry.ipv4_mac = Some(mac);
                }
                push_v4(&mut entry.v4, v4);
            }
            IpAddr::V6(v6) => {
                if let Some(mac) = neighbor_tables.ipv6.get(&(r.ifindex, v6)).copied() {
                    entry.ipv6_mac = Some(mac);
                }
                push_v6(&mut entry.v6, v6);
            }
        }
    }

    // Shape the final output: ifindex -> NetworkDevice
    let mut out: HashMap<u32, NetworkDevice> = HashMap::new();
    for (ifindex, a) in acc {
        // Keep IPv4 compatibility when the two gateway families resolve differently.
        let mac = a.ipv4_mac.or(a.ipv6_mac).unwrap_or_else(MacAddr::zero);
        out.insert(
            ifindex,
            NetworkDevice {
                mac_addr: mac,
                ipv4: a.v4,
                ipv6: a.v6,
            },
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_IFINDEX: u32 = 14;
    const TEST_MAC: [u8; 6] = [0x54, 0x9b, 0x49, 0x87, 0xe3, 0x48];

    fn push_sockaddr_in6(buf: &mut Vec<u8>, ip: Ipv6Addr, scope_id: u32) {
        let sockaddr_len = mem::size_of::<libc::sockaddr_in6>();
        let padded_len = roundup(sockaddr_len);
        let start = buf.len();
        buf.resize(start + padded_len, 0);
        buf[start] = sockaddr_len as u8;
        buf[start + 1] = libc::AF_INET6 as u8;
        buf[start + 8..start + 24].copy_from_slice(&ip.octets());
        buf[start + 24..start + 28].copy_from_slice(&scope_id.to_ne_bytes());
    }

    fn push_sockaddr_dl(buf: &mut Vec<u8>, ifindex: u16, mac: Option<[u8; 6]>) {
        let sockaddr_len = SOCKADDR_DL_HEADER_LEN + mac.map_or(0, |m| m.len());
        let padded_len = roundup(sockaddr_len);
        let start = buf.len();
        buf.resize(start + padded_len, 0);
        buf[start] = sockaddr_len as u8;
        buf[start + 1] = libc::AF_LINK as u8;
        buf[start + 2..start + 4].copy_from_slice(&ifindex.to_ne_bytes());
        if let Some(mac) = mac {
            buf[start + 6] = mac.len() as u8;
            buf[start + SOCKADDR_DL_HEADER_LEN..start + sockaddr_len].copy_from_slice(&mac);
        }
    }

    fn default_route(gateway: IpAddr, ifindex: u32) -> RawRoute {
        RawRoute {
            dst: match gateway {
                IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            },
            prefix: 0,
            gateway: Some(gateway),
            ifindex,
            flags: libc::RTF_GATEWAY,
        }
    }

    #[test]
    fn rejects_truncated_ip_sockaddrs() {
        for (family, length) in [(libc::AF_INET, 16usize), (libc::AF_INET6, 28usize)] {
            for truncated_length in 2..length {
                let mut bytes = vec![0; truncated_length];
                bytes[0] = truncated_length as u8;
                bytes[1] = family as u8;
                assert_eq!(
                    ip_from_sockaddr(&bytes),
                    None,
                    "family={family}, length={truncated_length}"
                );
            }
            let mut bytes = vec![0; length];
            bytes[0] = length as u8;
            bytes[1] = family as u8;
            assert!(ip_from_sockaddr(&bytes).is_some());
        }
    }

    #[test]
    fn parses_default_route_with_short_zero_mask() {
        let mut header: rt_msghdr = unsafe { mem::zeroed() };
        header.rtm_addrs = (1 << RTAX_DST) | (1 << RTAX_GATEWAY) | (1 << RTAX_NETMASK);
        header.rtm_index = 2;
        let mut bytes = vec![0; roundup(16) * 2 + roundup(0)];
        bytes[0] = 16;
        bytes[1] = libc::AF_INET as u8;
        let gateway_offset = roundup(16);
        bytes[gateway_offset] = 16;
        bytes[gateway_offset + 1] = libc::AF_INET as u8;
        bytes[gateway_offset + 4..gateway_offset + 8].copy_from_slice(&[192, 0, 2, 1]);
        let route = parse_one_route(&header, &bytes).unwrap();
        assert_eq!(route.prefix, 0);
        assert_eq!(route.gateway, Some("192.0.2.1".parse().unwrap()));
        bytes.pop();
        assert!(parse_one_route(&header, &bytes).is_none());
    }

    #[test]
    fn parses_ipv6_ndp_entry() {
        let ip: Ipv6Addr = "fe80::1".parse().unwrap();
        let mut message = Vec::new();
        push_sockaddr_in6(&mut message, ip, TEST_IFINDEX);
        push_sockaddr_dl(&mut message, TEST_IFINDEX as u16, Some(TEST_MAC));

        assert_eq!(
            message_to_neighbor(&message, 0),
            Some(NeighborEntry {
                ip: IpAddr::V6(ip),
                ifindex: TEST_IFINDEX,
                mac: MacAddr::from_octets(TEST_MAC),
            })
        );
    }

    #[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
    #[test]
    fn normalizes_kame_embedded_scope() {
        let embedded_scope: Ipv6Addr = "fe80:e::1".parse().unwrap();
        let mut message = Vec::new();
        push_sockaddr_in6(&mut message, embedded_scope, 0);
        push_sockaddr_dl(&mut message, 0, Some(TEST_MAC));

        let entry = message_to_neighbor(&message, 2).unwrap();
        assert_eq!(entry.ip, IpAddr::V6("fe80::1".parse().unwrap()));
        assert_eq!(entry.ifindex, TEST_IFINDEX);
    }

    #[test]
    fn keeps_ipv4_neighbors_separate_by_interface() {
        let ip = Ipv4Addr::new(192, 168, 1, 1);
        let first_mac = MacAddr::from_octets([0, 1, 2, 3, 4, 5]);
        let second_mac = MacAddr::from_octets([6, 7, 8, 9, 10, 11]);
        let mut tables = NeighborTables::default();
        tables.ipv4.insert((2, ip), first_mac);
        tables.ipv4.insert((3, ip), second_mac);
        let gateways = build_gateway_map(
            vec![
                default_route(IpAddr::V4(ip), 2),
                default_route(IpAddr::V4(ip), 3),
                default_route(IpAddr::V4(ip), 4),
            ],
            &tables,
        );
        assert_eq!(gateways[&2].mac_addr, first_mac);
        assert_eq!(gateways[&3].mac_addr, second_mac);
        assert_eq!(gateways[&4].mac_addr, MacAddr::zero());
    }

    #[test]
    fn parses_short_nonzero_masks() {
        assert_eq!(
            masklen_from_sockaddr(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                &[7, 0, 0, 0, 255, 255, 255]
            ),
            24
        );
        assert_eq!(
            masklen_from_sockaddr(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                &[10, 0, 0, 0, 0, 0, 0, 0, 255, 255]
            ),
            16
        );
    }

    #[test]
    fn keeps_link_local_neighbors_separate_by_interface() {
        let ip: Ipv6Addr = "fe80::1".parse().unwrap();
        let other_mac = MacAddr::from_octets([0, 1, 2, 3, 4, 5]);
        let mut tables = NeighborTables::default();
        tables
            .ipv6
            .insert((TEST_IFINDEX, ip), MacAddr::from_octets(TEST_MAC));
        tables.ipv6.insert((15, ip), other_mac);

        let gateways = build_gateway_map(
            vec![
                default_route(IpAddr::V6(ip), TEST_IFINDEX),
                default_route(IpAddr::V6(ip), 15),
            ],
            &tables,
        );

        assert_eq!(
            gateways.get(&TEST_IFINDEX).unwrap().mac_addr,
            MacAddr::from_octets(TEST_MAC)
        );
        assert_eq!(gateways.get(&15).unwrap().mac_addr, other_mac);
    }

    #[test]
    fn resolves_ipv6_only_gateway_and_preserves_zero_fallback() {
        let resolved_ip: Ipv6Addr = "fe80::1".parse().unwrap();
        let unresolved_ip: Ipv6Addr = "fe80::2".parse().unwrap();
        let mut tables = NeighborTables::default();
        tables
            .ipv6
            .insert((TEST_IFINDEX, resolved_ip), MacAddr::from_octets(TEST_MAC));

        let gateways = build_gateway_map(
            vec![
                default_route(IpAddr::V6(resolved_ip), TEST_IFINDEX),
                default_route(IpAddr::V6(unresolved_ip), 15),
            ],
            &tables,
        );

        assert_eq!(
            gateways.get(&TEST_IFINDEX).unwrap().mac_addr,
            MacAddr::from_octets(TEST_MAC)
        );
        assert_eq!(gateways.get(&15).unwrap().mac_addr, MacAddr::zero());
    }

    #[test]
    fn prefers_ipv4_mac_when_both_families_resolve() {
        let ipv4 = Ipv4Addr::new(192, 168, 10, 1);
        let ipv6: Ipv6Addr = "fe80::1".parse().unwrap();
        let ipv4_mac = MacAddr::from_octets(TEST_MAC);
        let ipv6_mac = MacAddr::from_octets([0, 1, 2, 3, 4, 5]);
        let mut tables = NeighborTables::default();
        tables.ipv4.insert((TEST_IFINDEX, ipv4), ipv4_mac);
        tables.ipv6.insert((TEST_IFINDEX, ipv6), ipv6_mac);

        let gateways = build_gateway_map(
            vec![
                default_route(IpAddr::V6(ipv6), TEST_IFINDEX),
                default_route(IpAddr::V4(ipv4), TEST_IFINDEX),
            ],
            &tables,
        );

        let gateway = gateways.get(&TEST_IFINDEX).unwrap();
        assert_eq!(gateway.mac_addr, ipv4_mac);
        assert_eq!(gateway.ipv4, vec![ipv4]);
        assert_eq!(gateway.ipv6, vec![ipv6]);
    }
}
