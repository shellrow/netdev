#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::interface::types::InterfaceType;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn get_interface_type(addr_ref: &libc::ifaddrs) -> InterfaceType {
    use std::ffi::CStr;
    use std::os::raw::c_char;
    use std::str::from_utf8_unchecked;

    let c_str = addr_ref.ifa_name as *const c_char;
    let bytes = unsafe { CStr::from_ptr(c_str).to_bytes() };
    let name: String = unsafe { from_utf8_unchecked(bytes).to_owned() };
    #[cfg(target_os = "linux")]
    {
        crate::os::linux::sysfs::get_interface_type(&name)
    }
    #[cfg(target_os = "android")]
    {
        crate::os::android::sysfs::get_interface_type(&name)
    }
}

#[cfg(target_vendor = "apple")]
pub use crate::os::darwin::types::get_interface_type;

#[cfg(any(target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
pub use crate::os::bsd::types::get_interface_type;
