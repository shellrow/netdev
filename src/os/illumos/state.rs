use crate::interface::state::OperState;

use super::lifreq::{SIOCGLIFFLAGS, get_lifreq};

/// Reads the current operational state of `if_name` by issuing a fresh `SIOCGLIFFLAGS`
/// and deriving state from `IFF_UP`/`IFF_RUNNING`, same as the BSD backend.
///
/// Tries an `AF_INET` socket first, then `AF_INET6`, since a v6-only logical interface
/// name may not resolve over an `AF_INET` one.
pub fn operstate(if_name: &str) -> OperState {
    let flags = get_lifreq(libc::AF_INET, if_name, SIOCGLIFFLAGS)
        .or_else(|| get_lifreq(libc::AF_INET6, if_name, SIOCGLIFFLAGS))
        .map(|req| unsafe { req.flags() });

    match flags {
        // Only the low-order IFF_UP / IFF_RUNNING bits matter to `from_if_flags`, both of
        // which fit safely in the truncated u32.
        Some(f) => OperState::from_if_flags(f as u32),
        None => OperState::Unknown,
    }
}
