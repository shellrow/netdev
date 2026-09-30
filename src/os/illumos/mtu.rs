use super::lifreq::{SIOCGLIFMTU, get_lifreq};

/// Reads the MTU of a logical interface via `SIOCGLIFMTU`.
///
/// illumos doesn't attach an MTU to the `getifaddrs` entry itself (`ifa` is unused here,
/// unlike the BSD/Darwin backends which pull it out of `if_data`), so this issues its own
/// ioctl. The query is tried over an `AF_INET` socket first, then `AF_INET6`, since a
/// v6-only logical interface may not resolve over an `AF_INET` one (and vice versa).
pub(crate) fn get_mtu(_ifa: &libc::ifaddrs, name: &str) -> Option<u32> {
    get_lifreq(libc::AF_INET, name, SIOCGLIFMTU)
        .or_else(|| get_lifreq(libc::AF_INET6, name, SIOCGLIFMTU))
        .map(|req| unsafe { req.mtu() })
}
