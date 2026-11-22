#[cfg(all(
    feature = "gateway",
    not(any(target_os = "ios", target_os = "illumos"))
))]
pub mod dns;
pub mod flags;
pub mod interface;
mod link_speed;
pub mod sockaddr;
pub mod types;

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub(crate) fn link_data(ifa: &libc::ifaddrs) -> Option<&libc::if_data> {
    if ifa.ifa_addr.is_null() || unsafe { (*ifa.ifa_addr).sa_family as i32 } != libc::AF_LINK {
        return None;
    }
    // getifaddrs supplies if_data for the link-layer entry.
    unsafe { ifa.ifa_data.cast::<libc::if_data>().as_ref() }
}
