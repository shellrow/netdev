use crate::interface::types::InterfaceType;

/// Best-effort interface type classification for illumos.
///
/// Unlike the BSD family, illumos' `getifaddrs` doesn't attach a BSD-style `if_data` blob
/// carrying a media/type code to the link-layer (`AF_LINK`) entry, and there's no single
/// cheap userland call that returns one uniformly across drivers (`e1000g`, `igb`,
/// `ixgbe`, `vioif`, `vnic`, link aggregations, ...). Loopback is reported precisely from
/// the interface flags already read via `getifaddrs`; everything else is reported as
/// `Ethernet`, which is correct for the large majority of illumos NICs and virtual NICs.
pub fn get_interface_type(addr_ref: &libc::ifaddrs) -> InterfaceType {
    if addr_ref.ifa_flags & (libc::IFF_LOOPBACK as u64) != 0 {
        InterfaceType::Loopback
    } else {
        InterfaceType::Ethernet
    }
}
