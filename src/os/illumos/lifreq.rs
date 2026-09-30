//! `struct lifreq` and the `SIOCG*LIF*` ioctl numbers used to query per-logical-interface
//! flags and MTU on illumos.
//!
//! `libc` does not currently bind `struct lifreq` or these ioctls for `target_os =
//! "illumos"`, so both are reproduced here from the illumos-gate headers:
//! `usr/src/uts/common/sys/sockio.h` (ioctl numbers, as `_IOWR('i', num, struct lifreq)`)
//! and `usr/src/uts/common/sys/if.h` / `if_tcp(7P)` (the `lifreq` layout). Only the union
//! members this crate actually reads (`lifru_flags`, `lifru_mtu`) are given real names; the
//! rest of `lifr_lifru` is represented as a `[u8; 256]` sized to match its largest member
//! (`struct sockaddr_storage`), so `size_of::<Lifreq>()` -- which feeds directly into the
//! ioctl command numbers below, per illumos' `_IOWR` macro -- matches what the kernel
//! expects.
//!
//! Note: unlike every other `libc` unix target, illumos declares
//! `ioctl(fildes: c_int, request: c_int, ...)` -- the request code is a plain `c_int`, not
//! a `c_ulong`.

use std::ffi::CString;
use std::mem::size_of;
use std::os::raw::{c_char, c_int};

/// `LIFNAMSIZ` from `<net/if.h>`.
const LIFNAMSIZ: usize = 32;

#[repr(C)]
union LifrLifru {
    /// Stand-in for the union's largest member (`struct sockaddr_storage`), so the union
    /// -- and therefore the enclosing `Lifreq` -- has the right size and alignment even
    /// though we never read this variant directly.
    _addr: [u8; 256],
    /// `SIOC[GS]LIFFLAGS`.
    flags: u64,
    /// `SIOC[GS]LIFMTU`.
    mtu: u32,
}

#[repr(C)]
pub(crate) struct Lifreq {
    lifr_name: [c_char; LIFNAMSIZ],
    /// `lifr_lifru1` union (`lifru_addrlen` / `lifru_ppa`); unused by the ioctls we issue,
    /// kept only so later fields land at the right offset.
    _lifr_lifru1: c_int,
    lifr_lifru: LifrLifru,
}

impl Lifreq {
    fn for_name(name: &str) -> Option<Self> {
        let cname = CString::new(name).ok()?;
        let bytes = cname.as_bytes_with_nul();
        if bytes.len() > LIFNAMSIZ {
            return None;
        }
        // All-zero is a valid bit pattern for every field here (a zeroed union is simply
        // interpreted as `flags: 0` / `mtu: 0` until the kernel fills it in).
        let mut req: Self = unsafe { std::mem::zeroed() };
        for (dst, src) in req.lifr_name.iter_mut().zip(bytes.iter()) {
            *dst = *src as c_char;
        }
        Some(req)
    }

    /// # Safety
    /// Only valid to call after a successful `SIOCGLIFFLAGS` ioctl populated this request.
    pub(crate) unsafe fn flags(&self) -> u64 {
        unsafe { self.lifr_lifru.flags }
    }

    /// # Safety
    /// Only valid to call after a successful `SIOCGLIFMTU` ioctl populated this request.
    pub(crate) unsafe fn mtu(&self) -> u32 {
        unsafe { self.lifr_lifru.mtu }
    }
}

const IOC_INOUT: u32 = 0xC000_0000;
const IOCPARM_MASK: u32 = 0xff;

/// Mirrors illumos' `_IOWR(group, num, t)` macro from `<sys/ioccom.h>`.
const fn iowr(group: u8, num: u8, size: usize) -> c_int {
    (IOC_INOUT | (((size as u32) & IOCPARM_MASK) << 16) | ((group as u32) << 8) | (num as u32))
        as c_int
}

/// `SIOCGLIFFLAGS` -- get extended (64-bit) interface flags. `_IOWR('i', 117, struct lifreq)`.
pub(crate) const SIOCGLIFFLAGS: c_int = iowr(b'i', 117, size_of::<Lifreq>());
/// `SIOCGLIFMTU` -- get interface MTU. `_IOWR('i', 122, struct lifreq)`.
pub(crate) const SIOCGLIFMTU: c_int = iowr(b'i', 122, size_of::<Lifreq>());

/// Opens a throwaway datagram socket in address family `af`, issues `cmd` (one of the
/// `SIOCG*` constants above) against the named logical interface, and returns the
/// populated request on success.
///
/// `af` should match the address family of the logical interface being queried (`AF_INET`
/// for an IPv4-only name, `AF_INET6` for an IPv6-only one); callers that aren't sure can
/// try both.
pub(crate) fn get_lifreq(af: c_int, name: &str, cmd: c_int) -> Option<Lifreq> {
    let mut req = Lifreq::for_name(name)?;
    let fd = unsafe { libc::socket(af, libc::SOCK_DGRAM, 0) };
    if fd < 0 {
        return None;
    }
    let res = unsafe { libc::ioctl(fd, cmd, &mut req as *mut Lifreq) };
    unsafe { libc::close(fd) };
    if res < 0 { None } else { Some(req) }
}
