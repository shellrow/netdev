#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use libc;

pub type SockAddr = libc::sockaddr;
pub const AF_LINK: libc::c_int = 18;

const IF_NAMESIZE: usize = 16;
const IFNAMSIZ: usize = IF_NAMESIZE;
const IOC_IN: libc::c_ulong = 0x80000000;
const IOC_OUT: libc::c_ulong = 0x40000000;
const IOC_INOUT: libc::c_ulong = IOC_IN | IOC_OUT;
const IOCPARM_SHIFT: libc::c_ulong = 13;
const IOCPARM_MASK: libc::c_ulong = (1 << (IOCPARM_SHIFT as usize)) - 1;

const SIZEOF_IFREQ: libc::c_ulong = 32;
const SIZEOF_C_UINT: libc::c_ulong = 4;

#[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
const SIZEOF_C_LONG: libc::c_int = 8;

pub const BIOCSETIF: libc::c_ulong = IOC_IN | ((SIZEOF_IFREQ & IOCPARM_MASK) << 16usize) | (('B' as libc::c_ulong) << 8usize) | 108;
pub const BIOCIMMEDIATE: libc::c_ulong = IOC_IN | ((SIZEOF_C_UINT & IOCPARM_MASK) << 16) | (('B' as libc::c_ulong) << 8) | 112;
pub const BIOCGDLT: libc::c_ulong = IOC_OUT | ((SIZEOF_C_UINT & IOCPARM_MASK) << 16) | (('B' as libc::c_ulong) << 8) | 106;
pub const BIOCSBLEN: libc::c_ulong = IOC_INOUT | ((SIZEOF_C_UINT & IOCPARM_MASK) << 16) | (('B' as libc::c_ulong) << 8) | 102;
pub const BIOCSHDRCMPLT: libc::c_ulong = IOC_IN | ((SIZEOF_C_UINT & IOCPARM_MASK) << 16) | (('B' as libc::c_ulong) << 8) | 117;

#[cfg(target_os = "freebsd")]
pub const BIOCFEEDBACK: libc::c_ulong = IOC_IN | ((SIZEOF_C_UINT & IOCPARM_MASK) << 16) | (('B' as libc::c_ulong) << 8) | 124;

#[cfg(target_os = "netbsd")]
pub const BIOCFEEDBACK: libc::c_ulong = IOC_IN | ((SIZEOF_C_UINT & IOCPARM_MASK) << 16) | (('B' as libc::c_ulong) << 8) | 125;

pub const DLT_NULL: libc::c_uint = 0;

#[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
const BPF_ALIGNMENT: libc::c_int = SIZEOF_C_LONG;

#[cfg(any(target_os = "openbsd", target_os = "macos", target_os = "ios", windows))]
const BPF_ALIGNMENT: libc::c_int = 4;

pub fn BPF_WORDALIGN(x: isize) -> isize {
    let bpf_alignment = BPF_ALIGNMENT as isize;
    (x + (bpf_alignment - 1)) & !(bpf_alignment - 1)
}

pub struct ifreq {
    pub ifr_name: [libc::c_char; IFNAMSIZ],
    pub ifru_addr: SockAddr, 
}

#[cfg(any(target_os = "openbsd", target_os = "freebsd", target_os = "netbsd", target_os = "macos", target_os = "ios"))]
pub struct sockaddr_dl {
    pub sdl_len: libc::c_uchar,
    pub sdl_family: libc::c_uchar,
    pub sdl_index: libc::c_ushort,
    pub sdl_type: libc::c_uchar,
    pub sdl_nlen: libc::c_uchar,
    pub sdl_alen: libc::c_uchar,
    pub sdl_slen: libc::c_uchar,
    pub sdl_data: [libc::c_char; 46],
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "netbsd",
    all(any(target_os = "macos", target_os = "ios"), target_pointer_width = "32"),
    windows
))]
#[repr(C)]
pub struct bpf_hdr {
    pub bh_tstamp: libc::timeval,
    pub bh_caplen: u32,
    pub bh_datalen: u32,
    pub bh_hdrlen: libc::c_ushort,
}

pub struct timeval32 {
    pub tv_sec: i32,
    pub tv_usec: i32,
}

#[cfg(any(
    target_os = "openbsd",
    all(any(target_os = "macos", target_os = "ios"), target_pointer_width = "64")
))]
#[repr(C)]
pub struct bpf_hdr {
    pub bh_tstamp: timeval32,
    pub bh_caplen: u32,
    pub bh_datalen: u32,
    pub bh_hdrlen: libc::c_ushort,
}

#[cfg(not(windows))]
extern "C" {
    pub fn ioctl(d: libc::c_int, request: libc::c_ulong, ...) -> libc::c_int;
}
