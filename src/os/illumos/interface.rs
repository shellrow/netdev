use crate::{interface::interface::Interface, os::unix::interface::unix_interfaces};

/// Enumerates network interfaces on illumos-based systems.
///
/// This delegates entirely to the shared `getifaddrs`-based path in
/// `crate::os::unix::interface`; per-address MAC, netmask, MTU and IPv6 flag lookups are
/// filled in from there via the platform hooks in this module.
///
/// `Interface::gateway`, `Interface::dns_servers` and `Interface::default` are left at
/// their defaults here even when the `gateway` feature is enabled -- default-route
/// discovery on illumos needs a `PF_ROUTE` routing-socket query that hasn't been ported
/// yet, so `netdev::get_default_gateway()` will return `Err` until that lands.
/// `netdev::get_default_interface()` still works, since it falls back to matching the
/// locally-routed source IP against each interface's addresses.
pub fn interfaces() -> Vec<Interface> {
    unix_interfaces()
}
