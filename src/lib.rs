#[cfg(not(target_os="windows"))]
mod sys;
#[cfg(any(target_os = "macos", target_os = "openbsd", target_os = "freebsd", target_os = "netbsd", target_os = "ios"))]
mod bpf;
#[cfg(any(target_os = "macos", target_os = "openbsd", target_os = "freebsd", target_os = "netbsd", target_os = "ios"))]
mod socket;

pub mod interface;
pub mod gateway;

pub use interface::Interface;
pub use interface::get_default_interface;
pub use interface::get_interfaces;
pub use gateway::Gateway;
pub use gateway::get_default_gateway;
