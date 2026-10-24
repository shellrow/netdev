//! illumos / Solaris-derivative backend (illumos, OmniOS, SmartOS, ...).
//!
//! Interface enumeration reuses the shared `getifaddrs`-based path in
//! `crate::os::unix::interface`. The pieces that differ from the BSD family enough to need
//! their own implementation live here: `struct lifreq`-based MTU and flag queries (`libc`
//! does not currently expose `lifreq` or the `SIOC*LIF*` ioctls for this target), and an
//! interface-type guess that doesn't depend on a BSD-style `if_data` blob.
//!
//! Not yet implemented: default-gateway / DNS discovery (`Interface::gateway`,
//! `Interface::dns_servers`, `Interface::default` stay at their defaults even with the
//! `gateway` feature enabled) and native traffic counters (`Interface::stats` stays `None`).
//! Both would need a routing-socket (`PF_ROUTE`) or `kstat(3KSTAT)` integration that hasn't
//! been ported yet.

pub mod flags;
pub mod interface;
pub mod ipv6_addr_flags;
mod lifreq;
pub mod mtu;
pub mod state;
pub mod types;
