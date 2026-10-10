use std::net::Ipv6Addr;

use crate::interface::ipv6_addr_flags::Ipv6AddrFlags;

// <netinet6/in6_var.h> — not yet in `libc`.
const SIOCGIFAFLAG_IN6: libc::c_ulong = 0xC1206949;
const IN6_IFF_TENTATIVE: u32 = 0x02;
const IN6_IFF_DUPLICATED: u32 = 0x04;
const IN6_IFF_DEPRECATED: u32 = 0x10;
const IN6_IFF_TEMPORARY: u32 = 0x80;

// `libc` does not expose `in6_ifreq` on FreeBSD/OpenBSD/NetBSD.
//
// Mirrors `struct in6_ifreq` from <netinet6/in6_var.h>. `SIOCGIFAFLAG_IN6` encodes
// `sizeof(struct in6_ifreq)` (0x120 = 288 bytes on LP64), and the kernel copies that many
// bytes in and out, so the union must be padded to the size of its largest member
// (`icmp6_ifstat`, 34 x u64 = 272 bytes). The flags are returned in `ifru_flags6`, which
// overlaps `ifru_addr` at the start of the union.
#[repr(C)]
struct In6Ifreq {
    ifr_name: [u8; libc::IFNAMSIZ],
    ifr_ifru: In6IfreqUnion,
}

#[repr(C)]
union In6IfreqUnion {
    ifru_addr: libc::sockaddr_in6,
    ifru_flags6: libc::c_int,
    // 8-byte aligned, like the u64 counters in `in6_ifstat` / `icmp6_ifstat`.
    _size: [u64; 34],
}

pub(crate) fn get_ipv6_addr_flags(ifname: &str, addr: &Ipv6Addr) -> Ipv6AddrFlags {
    unsafe {
        let fd = libc::socket(libc::AF_INET6, libc::SOCK_DGRAM, 0);
        if fd < 0 {
            return Ipv6AddrFlags::default();
        }

        let mut req: In6Ifreq = std::mem::zeroed();
        const _: () = assert!(std::mem::size_of::<In6Ifreq>() == 0x120);

        let name_bytes = ifname.as_bytes();
        let copy_len = name_bytes.len().min(libc::IFNAMSIZ - 1);
        std::ptr::copy_nonoverlapping(
            name_bytes.as_ptr(),
            req.ifr_name.as_mut_ptr().cast(),
            copy_len,
        );

        req.ifr_ifru.ifru_addr.sin6_family = libc::AF_INET6 as libc::sa_family_t;
        req.ifr_ifru.ifru_addr.sin6_len = std::mem::size_of::<libc::sockaddr_in6>() as u8;
        req.ifr_ifru.ifru_addr.sin6_addr.s6_addr = addr.octets();

        let ret = libc::ioctl(fd, SIOCGIFAFLAG_IN6, &mut req);
        libc::close(fd);

        if ret < 0 {
            return Ipv6AddrFlags::default();
        }

        let raw = req.ifr_ifru.ifru_flags6 as u32;

        Ipv6AddrFlags {
            deprecated: raw & IN6_IFF_DEPRECATED != 0,
            temporary: raw & IN6_IFF_TEMPORARY != 0,
            tentative: raw & IN6_IFF_TENTATIVE != 0,
            duplicated: raw & IN6_IFF_DUPLICATED != 0,
            permanent: false,
        }
    }
}
