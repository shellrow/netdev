use std::net::Ipv6Addr;

use crate::interface::ipv6_addr_flags::Ipv6AddrFlags;

use super::lifreq::{SIOCGLIFFLAGS, get_lifreq};

// From <sys/sockio.h> / if_tcp(7P); not yet exposed by `libc` for illumos. Unlike the BSD
// family (which has a dedicated `SIOCGIFAFLAG_IN6` ioctl for per-address state), illumos
// folds address-state bits directly into the owning logical interface's own extended
// flags, since each address normally lives on its own logical interface
// (`net0`, `net0:1`, `net0:2`, ...).
const IFF_DEPRECATED: u64 = 0x0000040000;
const IFF_TEMPORARY: u64 = 0x0800000000;
const IFF_DUPLICATE: u64 = 0x4000000000;

/// Reads IPv6 address-state flags for the logical interface named `ifname` via
/// `SIOCGLIFFLAGS`.
///
/// `ifname` is expected to be the exact logical interface that carries `addr` (as passed
/// down from `getifaddrs` in `crate::os::unix::interface::unix_interfaces`); `addr` itself
/// isn't needed for the lookup, since illumos keys these flags by interface name rather
/// than by address.
pub(crate) fn get_ipv6_addr_flags(ifname: &str, _addr: &Ipv6Addr) -> Ipv6AddrFlags {
    let flags = get_lifreq(libc::AF_INET6, ifname, SIOCGLIFFLAGS)
        .map(|req| unsafe { req.flags() })
        .unwrap_or(0);

    Ipv6AddrFlags {
        deprecated: flags & IFF_DEPRECATED != 0,
        temporary: flags & IFF_TEMPORARY != 0,
        // illumos doesn't expose a distinct "tentative" (DAD in progress) bit through
        // SIOCGLIFFLAGS -- addresses simply aren't reported by getifaddrs until DAD
        // completes -- so this is always `false` here.
        tentative: false,
        duplicated: flags & IFF_DUPLICATE != 0,
        // IFF_DEPRECATED / IFF_TEMPORARY / IFF_DUPLICATE are the only address-state bits
        // illumos exposes this way; there's no equivalent of Linux's IFA_F_PERMANENT.
        permanent: false,
    }
}
