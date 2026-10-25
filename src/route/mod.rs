use crate::interface::interface::Interface;
use crate::net::device::NetworkDevice;

/// Returns the default gateway associated with the active default interface.
///
/// Uses the same interface selection as [`crate::get_default_interface`]. Returns an error
/// when no default interface can be determined or its gateway information is unavailable.
pub fn get_default_gateway() -> Result<NetworkDevice, String> {
    resolve_default_gateway(crate::interface::get_interfaces())
}

fn resolve_default_gateway(interfaces: Vec<Interface>) -> Result<NetworkDevice, String> {
    crate::interface::resolve_default_interface(interfaces)?
        .gateway
        .ok_or_else(|| String::from("Default Gateway not found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_platform_default_without_requiring_a_local_ip() {
        let mut default = Interface::dummy();
        default.index = 2;
        default.default = true;
        let mut gateway = NetworkDevice::new();
        gateway.ipv4.push("192.0.2.1".parse().unwrap());
        default.gateway = Some(gateway.clone());
        assert_eq!(
            resolve_default_gateway(vec![Interface::dummy(), default]),
            Ok(gateway)
        );
    }

    #[test]
    fn does_not_use_another_interfaces_gateway() {
        let mut other = Interface::dummy();
        other.gateway = Some(NetworkDevice::new());
        let mut default = Interface::dummy();
        default.index = 2;
        default.default = true;
        assert_eq!(
            resolve_default_gateway(vec![other, default]),
            Err("Default Gateway not found".into())
        );
    }
}
