use crate::interface::interface::Interface;

pub fn is_physical_interface(interface: &Interface) -> bool {
    interface.is_up() && interface.is_running() && !interface.is_tun() && !interface.is_loopback()
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
pub fn get_interface_flags(if_name: &str) -> std::io::Result<u32> {
    use libc::{AF_INET, SOCK_DGRAM, c_char, ioctl, socket};
    use std::mem;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::ptr;

    #[cfg(target_os = "netbsd")]
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct IfReq {
        ifr_name: [c_char; libc::IFNAMSIZ],
        ifru: IfReqData,
    }

    #[cfg(target_os = "netbsd")]
    #[repr(C)]
    #[derive(Copy, Clone)]
    union IfReqData {
        space: libc::sockaddr_storage,
        flags: libc::c_short,
    }

    #[cfg(not(target_os = "netbsd"))]
    use libc::ifreq as IfReq;

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
        #[cfg(any(target_vendor = "apple", target_os = "openbsd"))]
        {
            Ok(unsafe { ifr.ifr_ifru.ifru_flags as u16 as u32 })
        }

        #[cfg(target_os = "netbsd")]
        {
            Ok(u32::from(unsafe { ifr.ifru.flags } as u16))
        }

        #[cfg(not(any(target_vendor = "apple", target_os = "netbsd", target_os = "openbsd")))]
        {
            let [low, high] = unsafe { ifr.ifr_ifru.ifru_flags };
            Ok(u32::from(low as u16) | (u32::from(high as u16) << 16))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_loopback_flags() {
        let flags = super::get_interface_flags("lo0").unwrap();
        assert_ne!(flags & libc::IFF_LOOPBACK as u32, 0);
        assert_eq!(
            super::get_interface_flags("invalid\0name")
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
}
