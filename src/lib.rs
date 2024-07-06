pub mod device;
pub mod gateway;
pub mod interface;
pub mod ip;
pub mod mac;
mod sys;
mod db;

pub use device::NetworkDevice;
pub use gateway::get_default_gateway;
pub use interface::get_default_interface;
pub use interface::get_interfaces;
pub use interface::Interface;
