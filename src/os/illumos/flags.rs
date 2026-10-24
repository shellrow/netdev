use crate::interface::interface::Interface;

/// Best-effort physical-interface heuristic for illumos.
///
/// Mirrors the BSD heuristic: up, running, not a tunnel-like point-to-point interface,
/// and not loopback. illumos additionally has VNICs, IPMP interfaces, and other virtual
/// constructs that aren't distinguished by a dedicated flag bit readable from userland, so
/// those may still pass this check; `Interface::is_physical()` additionally filters by MAC
/// vendor (via `crate::net::db::oui`), which catches most of them.
pub fn is_physical_interface(interface: &Interface) -> bool {
    interface.is_up() && interface.is_running() && !interface.is_tun() && !interface.is_loopback()
}
