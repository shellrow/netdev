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
