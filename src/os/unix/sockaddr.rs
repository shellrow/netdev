use crate::net::mac::MacAddr;
use core::mem;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub(crate) enum SockaddrRef<'a> {
    V4(&'a libc::sockaddr_in),
    V6(&'a libc::sockaddr_in6),
}

impl<'a> SockaddrRef<'a> {
    /// # Safety
    /// - `sa` must point to a valid, readable memory region of at least `len` bytes.
    /// - `sa` must point to the beginning of a `sockaddr`-compatible structure
    pub(crate) unsafe fn from_raw(sa: *const libc::sockaddr, len: libc::socklen_t) -> Option<Self> {
        if sa.is_null() || (len as usize) < mem::offset_of!(libc::sockaddr, sa_data) {
            return None;
        }
        let family = unsafe { (*sa).sa_family as libc::c_int };

        match family {
            libc::AF_INET => {
                if len as usize >= mem::size_of::<libc::sockaddr_in>() {
                    let sa: &libc::sockaddr_in = unsafe { &*(sa as *const libc::sockaddr_in) };
                    Some(SockaddrRef::V4(sa))
                } else {
                    None
                }
            }
            libc::AF_INET6 => {
                if len as usize >= mem::size_of::<libc::sockaddr_in6>() {
                    let sa: &libc::sockaddr_in6 = unsafe { &*(sa as *const libc::sockaddr_in6) };
                    Some(SockaddrRef::V6(sa))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn to_ip(&self) -> IpAddr {
        match self {
            SockaddrRef::V4(sin) => {
                let addr = u32::from_be((*sin).sin_addr.s_addr as u32);
                IpAddr::V4(Ipv4Addr::from(addr))
            }
            SockaddrRef::V6(sin6) => {
                let bytes = (*sin6).sin6_addr.s6_addr;
                #[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
                let bytes = normalize_kame_scope(bytes, sin6.sin6_scope_id).0;
                IpAddr::V6(Ipv6Addr::from(bytes))
            }
        }
    }

    #[inline]
    pub(crate) fn to_ipv6_scope(&self) -> Option<u32> {
        match self {
            SockaddrRef::V4(_) => None,
            SockaddrRef::V6(sin6) => {
                let scope = (*sin6).sin6_scope_id as u32;
                #[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
                let scope = normalize_kame_scope(sin6.sin6_addr.s6_addr, scope).1;
                Some(scope)
            }
        }
    }
}

#[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
fn normalize_kame_scope(mut bytes: [u8; 16], scope: u32) -> ([u8; 16], u32) {
    // getifaddrs can return the interface index inside a link-local address.
    // Expose the normal address and carry the zone only in ipv6_scope_ids.
    if Ipv6Addr::from(bytes).is_unicast_link_local() {
        let embedded_scope = u32::from(u16::from_be_bytes([bytes[2], bytes[3]]));
        bytes[2] = 0;
        bytes[3] = 0;
        (bytes, if scope == 0 { embedded_scope } else { scope })
    } else {
        (bytes, scope)
    }
}

#[cfg(all(test, any(target_os = "netbsd", target_os = "openbsd")))]
mod scope_tests {
    use super::*;

    #[test]
    fn parses_compact_unspecified_family_netmasks() {
        for (mask, expected) in [
            (&[5u8, 0, 0, 0, 255][..], "255.0.0.0"),
            (&[7u8, 0, 0, 0, 255, 255, 255][..], "255.255.255.0"),
        ] {
            let address = Some("192.0.2.1".parse().unwrap());
            let parsed = unsafe { netmask_ip_autolen(mask.as_ptr().cast(), address) };
            assert_eq!(parsed, Some(expected.parse().unwrap()));
            assert_eq!(
                unsafe { netmask_ip_autolen(mask.as_ptr().cast(), None) },
                None
            );
        }
        let mask = [10u8, 0, 0, 0, 0, 0, 0, 0, 255, 255];
        let parsed = unsafe {
            netmask_ip_autolen(mask.as_ptr().cast(), Some("2001:db8::1".parse().unwrap()))
        };
        assert_eq!(parsed, Some("ffff::".parse().unwrap()));
    }

    #[test]
    fn normalizes_link_local_addresses_and_preserves_scope() {
        let mut addr: libc::sockaddr_in6 = unsafe { mem::zeroed() };
        addr.sin6_addr.s6_addr = "fe80:1234::1".parse::<Ipv6Addr>().unwrap().octets();
        assert_eq!(
            SockaddrRef::V6(&addr).to_ip(),
            "fe80::1".parse::<IpAddr>().unwrap()
        );
        assert_eq!(SockaddrRef::V6(&addr).to_ipv6_scope(), Some(0x1234));
        addr.sin6_scope_id = 7;
        assert_eq!(SockaddrRef::V6(&addr).to_ipv6_scope(), Some(7));
        addr.sin6_addr.s6_addr = "fe80::1".parse::<Ipv6Addr>().unwrap().octets();
        assert_eq!(SockaddrRef::V6(&addr).to_ipv6_scope(), Some(7));
    }

    #[test]
    fn preserves_global_addresses_and_netmasks() {
        let mut addr: libc::sockaddr_in6 = unsafe { mem::zeroed() };
        for ip in ["2001:db8::1", "::1", "ffff:ffff:ffff:ffff::"] {
            let ip = ip.parse::<Ipv6Addr>().unwrap();
            addr.sin6_addr.s6_addr = ip.octets();
            assert_eq!(SockaddrRef::V6(&addr).to_ip(), IpAddr::V6(ip));
            assert_eq!(SockaddrRef::V6(&addr).to_ipv6_scope(), Some(0));
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) unsafe fn try_mac_from_raw(
    sa: *const libc::sockaddr,
    len: libc::socklen_t,
) -> Option<MacAddr> {
    if sa.is_null() || (len as usize) < mem::offset_of!(libc::sockaddr, sa_data) {
        return None;
    }

    let family = unsafe { (*sa).sa_family as libc::c_int };
    if family != libc::AF_PACKET {
        return None;
    }

    if (len as usize) < core::mem::size_of::<libc::sockaddr_ll>() {
        return None;
    }

    let sll = unsafe { &*(sa as *const libc::sockaddr_ll) };

    let halen = sll.sll_halen as usize;
    if halen != 6 {
        return None;
    }

    Some(MacAddr::from_octets([
        sll.sll_addr[0],
        sll.sll_addr[1],
        sll.sll_addr[2],
        sll.sll_addr[3],
        sll.sll_addr[4],
        sll.sll_addr[5],
    ]))
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub(crate) unsafe fn try_mac_from_raw(
    sa: *const libc::sockaddr,
    len: libc::socklen_t,
) -> Option<MacAddr> {
    // Validate pointer and minimal length (must contain sa_family)
    if sa.is_null() || (len as usize) < mem::offset_of!(libc::sockaddr, sa_data) {
        return None;
    }

    if unsafe { (*sa).sa_family } as libc::c_int != libc::AF_LINK {
        return None;
    }

    // Length sanity check
    let sa_len = unsafe { (*sa).sa_len } as usize;
    if sa_len == 0 {
        return None;
    }
    let eff_len = core::cmp::min(sa_len, len as usize);
    if eff_len < 8 {
        return None;
    }

    // Access sa_data region (starts after sa_len + sa_family)
    let base = sa as *const u8;
    let sa_data = unsafe { base.add(2) };
    // Offsets inside sockaddr_dl
    let nlen = unsafe { *sa_data.add(3) } as usize;
    let alen = unsafe { *sa_data.add(4) } as usize;

    if alen != 6 {
        return None;
    }
    // LLADDR is located at an offset of 6 + nlen bytes from the beginning of sa_data
    // (6 bytes correspond to index(2) + type(1) + nlen(1) + alen(1) + slen(1)).
    let lladdr_off_in_data = 6usize + nlen;
    let mac_end_in_data = lladdr_off_in_data + 6;

    // The usable length of sa_data is (eff_len - 2), excluding sa_len and sa_family.
    if mac_end_in_data > eff_len.saturating_sub(2) {
        return None;
    }

    let mac_ptr = unsafe { sa_data.add(lladdr_off_in_data) };
    unsafe {
        Some(MacAddr(
            *mac_ptr.add(0),
            *mac_ptr.add(1),
            *mac_ptr.add(2),
            *mac_ptr.add(3),
            *mac_ptr.add(4),
            *mac_ptr.add(5),
        ))
    }
}

// illumos has `AF_LINK`/`sockaddr_dl` like the BSD family, but its `sockaddr` has no
// `sa_len` field, and `libc`'s `sockaddr_dl` for this target is a proper named struct
// rather than an opaque byte blob, so this reads the fields directly instead of doing
// BSD's raw offset arithmetic above.
#[cfg(target_os = "illumos")]
pub(crate) unsafe fn try_mac_from_raw(
    sa: *const libc::sockaddr,
    len: libc::socklen_t,
) -> Option<MacAddr> {
    if sa.is_null() || (len as usize) < mem::offset_of!(libc::sockaddr, sa_data) {
        return None;
    }

    let family = unsafe { (*sa).sa_family as libc::c_int };
    if family != libc::AF_LINK {
        return None;
    }
    if (len as usize) < core::mem::size_of::<libc::sockaddr_dl>() {
        return None;
    }

    let sdl = unsafe { &*(sa as *const libc::sockaddr_dl) };
    let nlen = sdl.sdl_nlen as usize;
    let alen = sdl.sdl_alen as usize;
    if alen != 6 {
        return None;
    }
    // `sdl_data` holds the interface name (`sdl_nlen` bytes) immediately followed by the
    // link-layer address (`sdl_alen` bytes).
    if nlen + 6 > sdl.sdl_data.len() {
        return None;
    }

    let octet = |i: usize| sdl.sdl_data[nlen + i] as u8;
    Some(MacAddr::from_octets([
        octet(0),
        octet(1),
        octet(2),
        octet(3),
        octet(4),
        octet(5),
    ]))
}

/// Returns the length of an address supplied by getifaddrs.
#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub(crate) unsafe fn sockaddr_len(sa: *const libc::sockaddr) -> Option<libc::socklen_t> {
    if sa.is_null() {
        return None;
    }
    let len = unsafe { (*sa).sa_len };
    (len >= 2).then_some(len.into())
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "illumos"))]
pub(crate) unsafe fn sockaddr_len(sa: *const libc::sockaddr) -> Option<libc::socklen_t> {
    if sa.is_null() {
        return None;
    }
    let family = unsafe { (*sa).sa_family } as libc::c_int;
    // IPv4 / IPv6 are universally consistent across all OSes.
    if family == libc::AF_INET {
        return Some(core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t);
    }
    if family == libc::AF_INET6 {
        return Some(core::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t);
    }

    // Linux / Android: Layer 2 (AF_PACKET)
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if family == libc::AF_PACKET {
        return Some(core::mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t);
    }

    #[cfg(target_os = "illumos")]
    if family == libc::AF_LINK {
        return Some(core::mem::size_of::<libc::sockaddr_dl>() as libc::socklen_t);
    }

    // Unknown or unsupported family
    None
}

// illumos' `struct sockaddr` has no `sa_len` field either (it's SVR4-derived, not
// 4.4BSD-derived), so it shares this family-only implementation with Linux/Android rather
// than the sa_len-based one below.
#[cfg(any(target_os = "linux", target_os = "android", target_os = "illumos"))]
#[inline]
pub(crate) unsafe fn netmask_ip_autolen(
    sa: *const libc::sockaddr,
    _address: Option<IpAddr>,
) -> Option<IpAddr> {
    if sa.is_null() {
        return None;
    }

    match unsafe { (*sa).sa_family as libc::c_int } {
        libc::AF_INET => {
            let sin = unsafe { &*(sa as *const libc::sockaddr_in) };
            let n = u32::from_be(sin.sin_addr.s_addr as u32);
            Some(IpAddr::V4(Ipv4Addr::from(n)))
        }
        libc::AF_INET6 => {
            let sin6 = unsafe { &*(sa as *const libc::sockaddr_in6) };
            let bytes = sin6.sin6_addr.s6_addr; // [u8; 16]
            Some(IpAddr::V6(Ipv6Addr::from(bytes)))
        }
        _ => None,
    }
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
/// Reads a netmask `IpAddr` from a BSD/Darwin `sockaddr` whose actual length is encoded in `sa_len`.
#[inline]
pub(crate) unsafe fn netmask_ip_autolen(
    sa: *const libc::sockaddr,
    address: Option<IpAddr>,
) -> Option<IpAddr> {
    if sa.is_null() {
        return None;
    }

    // BSD/Darwin carry the effective length in the first byte (sa_len).
    let base = sa.cast::<u8>();
    let sa_len = unsafe { *base } as usize;
    if sa_len < 2 {
        return None;
    }

    let mut fam = unsafe { *base.add(1) } as libc::c_int;
    // OpenBSD returns compact IPv4 netmasks with AF_UNSPEC. Their family
    // must be taken from the associated interface address, not guessed by length.
    if fam == libc::AF_UNSPEC {
        fam = match address? {
            IpAddr::V4(_) => libc::AF_INET,
            IpAddr::V6(_) => libc::AF_INET6,
        };
    }

    match fam {
        libc::AF_INET => {
            // Layout on BSD/Darwin:
            //   offset 0: sa_len (1)
            //   offset 1: sa_family (1)
            //   offset 2..3: sin_port (2)
            //   offset 4..7: sin_addr (4)
            const OFF_SIN_ADDR: usize = 4;
            if sa_len <= OFF_SIN_ADDR {
                return Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
            }
            // Copy up to 4 bytes; tolerate short/truncated structures.
            let n = (sa_len - OFF_SIN_ADDR).min(4);

            let mut bytes = [0u8; 4];
            unsafe {
                core::ptr::copy_nonoverlapping(base.add(OFF_SIN_ADDR), bytes.as_mut_ptr(), n)
            };
            Some(IpAddr::V4(Ipv4Addr::from(u32::from_be_bytes(bytes))))
        }
        libc::AF_INET6 => {
            // Layout on BSD/Darwin:
            //   offset 0:  sa_len (1)
            //   offset 1:  sa_family (1)
            //   offset 2..3:  sin6_port (2)
            //   offset 4..7:  sin6_flowinfo (4)
            //   offset 8..23: sin6_addr (16)
            const OFF_SIN6_ADDR: usize = 8;
            if sa_len <= OFF_SIN6_ADDR {
                return Some(IpAddr::V6(Ipv6Addr::UNSPECIFIED));
            }
            // Copy up to 16 bytes; tolerate short/truncated structures.
            let n = (sa_len - OFF_SIN6_ADDR).min(16);

            let mut bytes = [0u8; 16];
            unsafe {
                core::ptr::copy_nonoverlapping(base.add(OFF_SIN6_ADDR), bytes.as_mut_ptr(), n)
            };
            Some(IpAddr::V6(Ipv6Addr::from(bytes)))
        }
        _ => None,
    }
}
