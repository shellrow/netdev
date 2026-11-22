use crate::interface::types::InterfaceType;

pub fn get_interface_type(addr_ref: &libc::ifaddrs) -> InterfaceType {
    crate::os::unix::link_data(addr_ref)
        .and_then(|data| InterfaceType::try_from(u32::from(data.ifi_type)).ok())
        .unwrap_or(InterfaceType::Unknown)
}
