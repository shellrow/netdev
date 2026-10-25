#![allow(non_camel_case_types)]

use libc::{c_int, c_uchar, pid_t, size_t};
use std::{
    collections::HashMap,
    ffi::c_void,
    io, mem,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
};

use crate::net::{device::NetworkDevice, mac::MacAddr};

const CTL_NET: c_int = 4;
#[allow(dead_code)]
const AF_ROUTE: c_int = 17;
const PF_ROUTE: c_int = 17;
const AF_LINK: c_int = 18;
const AF_INET: c_int = 2;
const AF_INET6: c_int = 30;

const NET_RT_DUMP: c_int = 1;
const NET_RT_FLAGS: c_int = 2;

const RTM_VERSION: c_uchar = 5;
const RTF_LLINFO: c_int = 1024;

// rtm_flags
#[allow(dead_code)]
const RTF_UP: c_int = 0x0001;
const RTF_GATEWAY: c_int = 0x0002;
#[allow(dead_code)]
const RTF_REJECT: c_int = 0x0008;
#[allow(dead_code)]
const RTF_STATIC: c_int = 0x0800;
const RTF_WASCLONED: c_int = 0x20000;

// rtm_addrs
const RTAX_DST: usize = 0;
const RTAX_GATEWAY: usize = 1;
const RTAX_NETMASK: usize = 2;
const RTAX_MAX: usize = 8;

// sockaddr alignment
const SA_ALIGN: usize = 4;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct rt_metrics {
    rmx_locks: u32,
    rmx_mtu: u32,
    rmx_hopcount: u32,
    rmx_expire: i32,
    rmx_recvpipe: u32,
    rmx_sendpipe: u32,
    rmx_ssthresh: u32,
    rmx_rtt: u32,
    rmx_rttvar: u32,
    rmx_pksent: u32,
    rmx_state: u32,
    rmx_filler: [u32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct rt_msghdr {
    rtm_msglen: u16,
    rtm_version: u8,
    rtm_type: u8,
    rtm_index: u16,
    rtm_flags: c_int,
    rtm_addrs: c_int,
    rtm_pid: pid_t,
    rtm_seq: c_int,
    rtm_errno: c_int,
    rtm_use: c_int,
    rtm_inits: u32,
    rtm_rmx: rt_metrics,
}

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
        SA_ALIGN
    } else {
        (len + (SA_ALIGN - 1)) & !(SA_ALIGN - 1)
    }
}

#[inline]
fn normalize_scoped_v6(gw: Ipv6Addr) -> Ipv6Addr {
    // Unicast link-local: fe80::/10 (in practice often fe80::/64)
    let oct = gw.octets();
    let is_unicast_ll = oct[0] == 0xfe && (oct[1] & 0xc0) == 0x80;

    // Multicast check (ff00::/8) and local scopes: 0x1 (node-local) or 0x2 (link-local).
    let is_multicast = oct[0] == 0xff;
    let mscope = oct[1] & 0x0f;
    let is_local_scope_mc = is_multicast && (mscope == 0x1 || mscope == 0x2);

    if is_unicast_ll || is_local_scope_mc {
        let segs = gw.segments();
        Ipv6Addr::new(
            segs[0], 0, segs[2], segs[3], segs[4], segs[5], segs[6], segs[7],
        )
    } else {
        gw
    }
}

#[inline]
fn normalize_gateway(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(v4) => IpAddr::V4(v4),
        IpAddr::V6(v6) => IpAddr::V6(normalize_scoped_v6(v6)),
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
        17 => io::ErrorKind::AlreadyExists, // EEXIST
        3 => io::ErrorKind::NotFound,       // ESRCH
        libc::ENOBUFS => io::ErrorKind::OutOfMemory,
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
                ip: normalize_gateway(ip?),
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

    // Walk `sockaddr` records while there is room for a header.
    while off + 2 <= msg.len() {
        let sa_len = msg[off] as usize;

        // `sa_len == 0` can appear as "no address" (alignment-only slot).
        // Advance by the platform's alignment unit (4 on BSD/Darwin).
        if sa_len == 0 {
            off += roundup(0);
            continue;
        }
        // If the element claims to extend past the buffer, skip it conservatively.
        if off + sa_len > msg.len() {
            off += roundup(sa_len);
            continue;
        }

        let sa = &msg[off..off + sa_len];
        if sa.len() < 2 {
            return None;
        }
        match sa[1] as c_int {
            AF_INET => {
                // Target IPv4 of ARP. `sockaddr_in` and `sockaddr_inarp` share the initial layout,
                // so `sin_addr` sits at the same position.
                ip = ip_from_sockaddr(sa);
                if let Some(entry) = make_entry(ip, mac, link_ifindex, scope_id) {
                    return Some(entry);
                }
            }
            AF_INET6 => {
                if let Some(parsed_ip @ IpAddr::V6(_)) = ip_from_sockaddr(sa) {
                    scope_id = u32::from_ne_bytes(sa.get(24..28)?.try_into().ok()?);
                    ip = Some(parsed_ip);
                    if let Some(entry) = make_entry(ip, mac, link_ifindex, scope_id) {
                        return Some(entry);
                    }
                }
            }
            AF_LINK => {
                if sa_len < mem::size_of::<libc::sockaddr_dl>() {
                    off += roundup(sa_len);
                    continue;
                }
                // Read the variable-length link address within its declared bounds.
                let sdl = &msg[off..off + sa_len];
                let nlen = sdl[5] as usize;
                let alen = sdl[6] as usize;
                link_ifindex = u16::from_ne_bytes([sdl[2], sdl[3]]) as u32;

                let mac_start = 8 + nlen;
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

        // Advance to the next record; BSD/Darwin sockaddrs are 4-byte aligned.
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

/// Read ARP or NDP entries from the Darwin routing table via `sysctl`.
fn get_neighbor_entries(address_family: c_int) -> io::Result<Vec<NeighborEntry>> {
    let mut entries = Vec::new();
    let mut mib = [
        CTL_NET,  // net
        PF_ROUTE, // route
        0,        // 0
        address_family,
        NET_RT_FLAGS, // flags
        RTF_LLINFO,   // ARP/neighbor entries
    ];
    // Includes ENOMEM retry internally; length is truncated to actual bytes read.
    let buf = sysctl_vec(&mut mib)?;

    let mut off = 0usize;
    // Each record starts with `rt_msghdr` followed by a variable-length sockaddr block.
    while off + mem::size_of::<rt_msghdr>() <= buf.len() {
        // Copy the header without requiring the byte buffer to be aligned.
        let hdr = unsafe { std::ptr::read_unaligned(buf[off..].as_ptr().cast::<rt_msghdr>()) };
        let msglen = hdr.rtm_msglen as usize;
        if msglen < mem::size_of::<rt_msghdr>() || off + msglen > buf.len() {
            break;
        }

        // Version mismatch: skip the record but keep reading.
        if hdr.rtm_version != RTM_VERSION {
            off += msglen;
            continue;
        }
        if hdr.rtm_errno != 0 {
            return Err(code_to_error(hdr.rtm_errno));
        }

        // Parse the sockaddr block right after the header.
        let addr_block = &buf[off + mem::size_of::<rt_msghdr>()..off + msglen];
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

    // Read the two families independently so an NDP failure cannot discard valid ARP data,
    // and an ARP failure cannot prevent IPv6-only gateways from being resolved.
    for entry in get_neighbor_entries(AF_INET).unwrap_or_default() {
        if let IpAddr::V4(ip) = entry.ip {
            tables.ipv4.insert((entry.ifindex, ip), entry.mac);
        }
    }
    for entry in get_neighbor_entries(AF_INET6).unwrap_or_default() {
        if let IpAddr::V6(ip) = entry.ip {
            tables.ipv6.insert((entry.ifindex, ip), entry.mac);
        }
    }

    tables
}

/// Dump the routing table via `sysctl` on BSD/Darwin and parse each `rt_msghdr`.
fn list_routes() -> io::Result<Vec<RawRoute>> {
    // sysctl net.route dump (all families)
    let mut mib = [
        CTL_NET,     // net
        PF_ROUTE,    // route
        0,           // 0
        0,           // all families (0)
        NET_RT_DUMP, // dump routes
        0,
    ];
    // Includes ENOMEM retry internally; buffer is right-sized and truncated.
    let buf = sysctl_vec(&mut mib)?;

    let mut out = Vec::<RawRoute>::new();
    let mut off = 0usize;

    while off + mem::size_of::<rt_msghdr>() <= buf.len() {
        let hdr = unsafe { std::ptr::read_unaligned(buf[off..].as_ptr().cast::<rt_msghdr>()) };
        let msglen = hdr.rtm_msglen as usize;
        if msglen < mem::size_of::<rt_msghdr>() || off + msglen > buf.len() {
            break;
        }
        // Version mismatch: skip the record but keep reading.
        if hdr.rtm_version != RTM_VERSION {
            off += msglen;
            continue;
        }
        // Skip cloned/old-style cache entries; keep only real routes.
        if (hdr.rtm_flags & RTF_WASCLONED) != 0 {
            off += msglen;
            continue;
        }
        if hdr.rtm_errno != 0 {
            return Err(code_to_error(hdr.rtm_errno));
        }

        let addr_block = &buf[off + mem::size_of::<rt_msghdr>()..off + msglen];
        if let Some(rr) = parse_one_route(&hdr, addr_block) {
            out.push(rr);
        }

        off += msglen;
    }

    Ok(out)
}

/// Build a map `ifindex -> NetworkDevice` for default gateways on BSD/Darwin.
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
        if r.prefix != 0 || (r.flags & RTF_GATEWAY) == 0 {
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
        // Preserve the existing IPv4 result when both families resolve to different addresses.
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

    fn push_sockaddr_in(buf: &mut Vec<u8>, ip: Ipv4Addr) {
        let mut sa = [0u8; 16];
        sa[0] = sa.len() as u8;
        sa[1] = AF_INET as u8;
        sa[4..8].copy_from_slice(&ip.octets());
        buf.extend_from_slice(&sa);
    }

    fn push_sockaddr_in6(buf: &mut Vec<u8>, ip: Ipv6Addr, scope_id: u32) {
        let mut sa = [0u8; 28];
        sa[0] = sa.len() as u8;
        sa[1] = AF_INET6 as u8;
        sa[8..24].copy_from_slice(&ip.octets());
        sa[24..28].copy_from_slice(&scope_id.to_ne_bytes());
        buf.extend_from_slice(&sa);
    }

    fn push_sockaddr_dl(buf: &mut Vec<u8>, ifindex: u16, mac: Option<[u8; 6]>) {
        let mut sa = [0u8; 20];
        sa[0] = sa.len() as u8;
        sa[1] = AF_LINK as u8;
        sa[2..4].copy_from_slice(&ifindex.to_ne_bytes());
        if let Some(mac) = mac {
            sa[6] = mac.len() as u8;
            sa[8..14].copy_from_slice(&mac);
        }
        buf.extend_from_slice(&sa);
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
            flags: RTF_GATEWAY,
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
    fn parses_ipv4_arp_entry() {
        let ip = Ipv4Addr::new(192, 168, 10, 1);
        let mut msg = Vec::new();
        push_sockaddr_in(&mut msg, ip);
        push_sockaddr_dl(&mut msg, TEST_IFINDEX as u16, Some(TEST_MAC));

        assert_eq!(
            message_to_neighbor(&msg, 0),
            Some(NeighborEntry {
                ip: IpAddr::V4(ip),
                ifindex: TEST_IFINDEX,
                mac: MacAddr::from_octets(TEST_MAC),
            })
        );
    }

    #[test]
    fn parses_scoped_ipv6_ndp_entry() {
        let embedded_scope: Ipv6Addr = "fe80:e::1".parse().unwrap();
        let mut msg = Vec::new();
        push_sockaddr_in6(&mut msg, embedded_scope, 0);
        push_sockaddr_dl(&mut msg, TEST_IFINDEX as u16, Some(TEST_MAC));

        assert_eq!(
            message_to_neighbor(&msg, 0),
            Some(NeighborEntry {
                ip: IpAddr::V6("fe80::1".parse().unwrap()),
                ifindex: TEST_IFINDEX,
                mac: MacAddr::from_octets(TEST_MAC),
            })
        );
    }

    #[test]
    fn uses_scope_id_when_link_index_is_missing() {
        let ip: Ipv6Addr = "fe80::1".parse().unwrap();
        let mut msg = Vec::new();
        push_sockaddr_in6(&mut msg, ip, TEST_IFINDEX);
        push_sockaddr_dl(&mut msg, 0, Some(TEST_MAC));

        assert_eq!(message_to_neighbor(&msg, 2).unwrap().ifindex, TEST_IFINDEX);
    }

    #[test]
    fn normalizes_the_full_ipv6_link_local_prefix() {
        let scoped: Ipv6Addr = "febf:e::1".parse().unwrap();
        assert_eq!(
            normalize_scoped_v6(scoped),
            "febf::1".parse::<Ipv6Addr>().unwrap()
        );
    }

    #[test]
    fn rejects_incomplete_and_truncated_neighbor_entries() {
        let ip: Ipv6Addr = "fe80::1".parse().unwrap();
        let mut incomplete = Vec::new();
        push_sockaddr_in6(&mut incomplete, ip, TEST_IFINDEX);
        push_sockaddr_dl(&mut incomplete, TEST_IFINDEX as u16, None);
        assert_eq!(message_to_neighbor(&incomplete, TEST_IFINDEX), None);

        let mut truncated = vec![16, AF_INET6 as u8];
        truncated.resize(16, 0);
        assert_eq!(message_to_neighbor(&truncated, TEST_IFINDEX), None);

        let mut truncated_link = Vec::new();
        push_sockaddr_in(&mut truncated_link, Ipv4Addr::new(192, 168, 10, 1));
        truncated_link.extend_from_slice(&[16, AF_LINK as u8, 0, 0, 0, 0, 6, 0]);
        truncated_link.resize(32, 0);
        assert_eq!(message_to_neighbor(&truncated_link, TEST_IFINDEX), None);
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
    fn prefers_existing_ipv4_mac_when_both_families_resolve() {
        let ipv4 = Ipv4Addr::new(192, 168, 10, 1);
        let ipv6: Ipv6Addr = "fe80::1".parse().unwrap();
        let ipv4_mac = MacAddr::from_octets(TEST_MAC);
        let ipv6_mac = MacAddr::from_octets([0, 1, 2, 3, 4, 5]);
        let mut tables = NeighborTables::default();
        tables.ipv4.insert((TEST_IFINDEX, ipv4), ipv4_mac);
        tables.ipv6.insert((TEST_IFINDEX, ipv6), ipv6_mac);

        let gateways = build_gateway_map(
            vec![
                default_route(IpAddr::V4(ipv4), TEST_IFINDEX),
                default_route(IpAddr::V6(ipv6), TEST_IFINDEX),
            ],
            &tables,
        );

        let gateway = gateways.get(&TEST_IFINDEX).unwrap();
        assert_eq!(gateway.mac_addr, ipv4_mac);
        assert_eq!(gateway.ipv4, vec![ipv4]);
        assert_eq!(gateway.ipv6, vec![ipv6]);
    }
}
