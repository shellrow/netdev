mod os;
pub mod interface;
pub mod gateway;

pub use interface::Interface;
pub use interface::get_default_interface;
pub use gateway::Gateway;
pub use gateway::get_default_gateway;
