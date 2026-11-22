pub(crate) fn get_mtu(ifa: &libc::ifaddrs, _name: &str) -> Option<u32> {
    crate::os::unix::link_data(ifa).map(|data| data.ifi_mtu as u32)
}
