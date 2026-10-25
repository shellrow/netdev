use std::net::Ipv6Addr;

use crate::interface::ipv6_addr_flags::Ipv6AddrFlags;

// <netinet6/in6_var.h> — not yet in `libc`.
const SIOCGIFAFLAG_IN6: libc::c_ulong = 0xC1206949;
const IN6_IFF_TENTATIVE: u32 = 0x02;
const IN6_IFF_DUPLICATED: u32 = 0x04;
const IN6_IFF_DEPRECATED: u32 = 0x10;
const IN6_IFF_TEMPORARY: u32 = 0x80;

// Match <netinet6/in6_var.h> on FreeBSD, OpenBSD, and NetBSD. The kernel
// copies the entire 288-byte request, even when only the address/flags are used.
#[repr(C)]
struct In6Ifreq {
    ifr_name: [u8; libc::IFNAMSIZ],
    ifr_ifru: In6IfreqUnion,
}

#[repr(C)]
union In6IfreqUnion {
    addr: libc::sockaddr_in6,
    flags6: libc::c_int,
    // icmp6_ifstat, the largest union member, contains 34 uint64_t counters.
    icmp6_stats: [u64; 34],
}

const _: [(); 288] = [(); std::mem::size_of::<In6Ifreq>()];

pub(crate) fn get_ipv6_addr_flags(ifname: &str, addr: &Ipv6Addr) -> Ipv6AddrFlags {
    unsafe {
        let fd = libc::socket(libc::AF_INET6, libc::SOCK_DGRAM, 0);
        if fd < 0 {
            return Ipv6AddrFlags::default();
        }

        let mut req: In6Ifreq = std::mem::zeroed();

        let name_bytes = ifname.as_bytes();
        let copy_len = name_bytes.len().min(libc::IFNAMSIZ - 1);
        std::ptr::copy_nonoverlapping(
            name_bytes.as_ptr(),
            req.ifr_name.as_mut_ptr().cast(),
            copy_len,
        );

        req.ifr_ifru.addr.sin6_family = libc::AF_INET6 as libc::sa_family_t;
        req.ifr_ifru.addr.sin6_len = std::mem::size_of::<libc::sockaddr_in6>() as u8;
        req.ifr_ifru.addr.sin6_addr.s6_addr = addr.octets();
        if addr.is_unicast_link_local() {
            req.ifr_ifru.addr.sin6_scope_id = libc::if_nametoindex(req.ifr_name.as_ptr().cast());
        }

        let ret = libc::ioctl(fd, SIOCGIFAFLAG_IN6, &mut req);
        libc::close(fd);

        if ret < 0 {
            return Ipv6AddrFlags::default();
        }

        let raw = req.ifr_ifru.flags6 as u32;

        Ipv6AddrFlags {
            deprecated: raw & IN6_IFF_DEPRECATED != 0,
            temporary: raw & IN6_IFF_TEMPORARY != 0,
            tentative: raw & IN6_IFF_TENTATIVE != 0,
            duplicated: raw & IN6_IFF_DUPLICATED != 0,
            permanent: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_matches_bsd_kernel_abi() {
        // Verified against the native headers on FreeBSD 14.4, OpenBSD 7.9,
        // and NetBSD 10.1. ioctl copies this size in both directions.
        assert_eq!(std::mem::size_of::<In6Ifreq>(), 288);
        assert_eq!(std::mem::offset_of!(In6Ifreq, ifr_ifru), 16);
        assert_eq!((SIOCGIFAFLAG_IN6 >> 16) & 0x1fff, 288);
        let mut req: In6Ifreq = unsafe { std::mem::zeroed() };
        req.ifr_ifru.flags6 = IN6_IFF_TEMPORARY as libc::c_int;
        let flags_ptr = &raw const req.ifr_ifru.flags6;
        let addr_ptr = &raw const req.ifr_ifru.addr;
        assert_eq!(flags_ptr.cast::<u8>(), addr_ptr.cast::<u8>());
    }
}
