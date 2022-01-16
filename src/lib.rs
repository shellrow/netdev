#[cfg(not(target_os="windows"))]
mod sys;
#[cfg(not(target_os="windows"))]
mod bpf;
#[cfg(not(target_os="windows"))]
mod socket;

pub mod interface;
pub mod gateway;

pub use interface::Interface;
pub use interface::get_default_interface;
pub use interface::get_interfaces;
pub use gateway::Gateway;
pub use gateway::get_default_gateway;
