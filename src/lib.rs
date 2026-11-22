//! Cross-platform library for enumerating network interfaces and their addresses, link properties, and traffic counters.
//!
//! [`get_interfaces`] returns a snapshot. Metadata availability depends on the OS and driver.
//!
//! ```
//! for interface in netdev::get_interfaces() {
//!     println!("{}: {:?}", interface.name, interface.ip_addrs_with_prefix());
//! }
//! ```
//!
#![cfg_attr(
    feature = "gateway",
    doc = "[`get_default_interface`] and [`get_default_gateway`] are available with the `gateway` feature (default)."
)]
#![cfg_attr(
    not(feature = "gateway"),
    doc = "`get_default_interface` and `get_default_gateway` are available with the `gateway` feature (default)."
)]
//!
//! On Android, the default `android-extra` feature requires an initialized `ndk-context`.
//! Standalone binaries can disable default features and enable `gateway` separately.

pub mod interface;
pub mod net;
mod os;
pub mod prelude;
#[cfg(feature = "gateway")]
pub mod route;
pub mod stats;

pub use ipnet;

pub use interface::get_interfaces;
pub use interface::interface::Interface;
pub use net::device::NetworkDevice;
pub use net::mac::MacAddr;

#[cfg(feature = "gateway")]
pub use interface::get_default_interface;
#[cfg(feature = "gateway")]
pub use route::get_default_gateway;
