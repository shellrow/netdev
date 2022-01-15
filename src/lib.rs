mod os;
mod sys;
mod bpf;
pub mod interface;
pub mod gateway;

pub use interface::Interface;
pub use interface::get_default_interface;
pub use interface::get_interfaces;
pub use gateway::Gateway;
pub use gateway::get_default_gateway;
