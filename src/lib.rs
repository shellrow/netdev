mod interface;
mod gateway;

pub use interface::Interface;
pub use interface::get_default_interface;
pub use interface::get_default_interface_index;
pub use interface::get_default_interface_name;

pub use gateway::Gateway;
pub use gateway::get_default_gateway;
pub use gateway::get_default_gateway_ip;
pub use gateway::get_default_gateway_mac;

