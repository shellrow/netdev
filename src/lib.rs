#[cfg(any(
    target_os = "macos",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "ios"
))]
mod bpf;
#[cfg(any(target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
mod socket;

pub mod gateway;
pub mod interface;
pub mod ip;
pub mod mac;
mod sys;

pub use gateway::get_default_gateway;
pub use gateway::Gateway;
pub use interface::get_default_interface;
pub use interface::get_interfaces;
pub use interface::Interface;
