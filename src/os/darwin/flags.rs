use crate::interface::interface::Interface;

pub fn is_physical_interface(interface: &Interface) -> bool {
    interface.is_up() && interface.is_running() && !interface.is_tun() && !interface.is_loopback()
}

pub fn get_interface_flags(if_name: &str) -> std::io::Result<u32> {
    use libc::ifreq as IfReq;
    use libc::{AF_INET, SOCK_DGRAM, c_char, ioctl, socket};
    use std::mem;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::ptr;

    const SIOCGIFFLAGS: libc::c_ulong =
        0xc0006911 | ((mem::size_of::<IfReq>() as libc::c_ulong) << 16);
    let mut ifr: IfReq = unsafe { mem::zeroed() };

    let ifname_c = std::ffi::CString::new(if_name).map_err(|_| std::io::ErrorKind::InvalidInput)?;
    let bytes = ifname_c.as_bytes_with_nul();

    if bytes.len() > ifr.ifr_name.len() {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }

    unsafe {
        ptr::copy_nonoverlapping(
            bytes.as_ptr() as *const c_char,
            ifr.ifr_name.as_mut_ptr(),
            bytes.len(),
        );
    }

    let fd = unsafe { socket(AF_INET, SOCK_DGRAM, 0) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let sock = unsafe { OwnedFd::from_raw_fd(fd) };
    let res = unsafe { ioctl(sock.as_raw_fd(), SIOCGIFFLAGS, &mut ifr) };
    if res < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { ifr.ifr_ifru.ifru_flags as u16 as u32 })
    }
}
