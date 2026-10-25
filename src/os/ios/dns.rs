//! DNS resolver lookup for iOS through libresolv.
//!
//! The SystemConfiguration dynamic store is `API_UNAVAILABLE(ios)`;
//! `res_ninit` / `res_getservers` are the public equivalent.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// `struct __res_state` from `<resolv.h>`, kept opaque: 552 bytes, 8-aligned
/// on 64-bit iOS.
#[repr(C, align(8))]
struct ResState([u8; 552]);

/// `union res_sockaddr_union` from `<resolv.h>`: 128 bytes, 4-aligned.
#[repr(C, align(4))]
#[derive(Clone, Copy)]
struct ResSockaddrUnion([u8; 128]);

/// `MAXNS` from `<resolv.h>`.
const MAXNS: usize = 3;

#[link(name = "resolv")]
unsafe extern "C" {
    fn res_9_ninit(state: *mut ResState) -> libc::c_int;
    fn res_9_getservers(
        state: *mut ResState,
        set: *mut ResSockaddrUnion,
        cnt: libc::c_int,
    ) -> libc::c_int;
    fn res_9_ndestroy(state: *mut ResState);
}

pub fn get_system_dns_conf() -> Vec<IpAddr> {
    let mut state = ResState([0; 552]);
    let mut servers = [ResSockaddrUnion([0; 128]); MAXNS];
    // SAFETY: `state` is zeroed and as large and aligned as `struct __res_state`,
    // as `res_ninit` requires, and destroyed only after a successful init.
    // `servers` holds `MAXNS` unions, the count passed.
    let count = unsafe {
        if res_9_ninit(&mut state) != 0 {
            return Vec::new();
        }
        let count = res_9_getservers(&mut state, servers.as_mut_ptr(), MAXNS as libc::c_int);
        res_9_ndestroy(&mut state);
        count
    };
    servers
        .iter()
        .take(usize::try_from(count).unwrap_or(0))
        .filter_map(sockaddr_ip)
        .collect()
}

fn sockaddr_ip(addr: &ResSockaddrUnion) -> Option<IpAddr> {
    // BSD `sockaddr`: `sa_len` then `sa_family`.
    match i32::from(addr.0[1]) {
        libc::AF_INET => {
            // SAFETY: the union holds a `sockaddr_in` for `AF_INET` and is larger than it.
            let sin: libc::sockaddr_in =
                unsafe { std::ptr::read_unaligned(addr.0.as_ptr().cast()) };
            Some(IpAddr::V4(Ipv4Addr::from(u32::from_be(
                sin.sin_addr.s_addr,
            ))))
        }
        libc::AF_INET6 => {
            // SAFETY: the union holds a `sockaddr_in6` for `AF_INET6` and is larger than it.
            let sin6: libc::sockaddr_in6 =
                unsafe { std::ptr::read_unaligned(addr.0.as_ptr().cast()) };
            Some(IpAddr::V6(Ipv6Addr::from(sin6.sin6_addr.s6_addr)))
        }
        _ => None,
    }
}
