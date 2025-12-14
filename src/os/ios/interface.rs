use crate::interface::types::InterfaceType;
use crate::os::darwin::types::{get_functional_type, interface_type_by_name};
use crate::{interface::interface::Interface, os::unix::interface::unix_interfaces};

pub fn interfaces() -> Vec<Interface> {
    let mut ifaces: Vec<Interface> = unix_interfaces();

    #[cfg(feature = "gateway")]
    let gateway_map = crate::os::darwin::route::get_gateway_map();

    for iface in &mut ifaces {
        // If interface type is Ethernet, try to get a more accurate type
        if iface.if_type == InterfaceType::Ethernet {
            let ft: InterfaceType = get_functional_type(&iface.name);
            if ft != InterfaceType::Unknown {
                iface.if_type = ft;
            }
        }

        if let Some(name_type) = interface_type_by_name(&iface.name) {
            iface.if_type = name_type;
        }

        #[cfg(feature = "gateway")]
        {
            if let Some(gateway) = gateway_map.get(&iface.index) {
                iface.gateway = Some(gateway.clone());
            }
        }
    }

    #[cfg(feature = "gateway")]
    {
        if let Some(local_ip) = crate::net::ip::get_local_ipaddr() {
            if let Some(idx) = crate::interface::pick_default_iface_index(&ifaces, local_ip) {
                if let Some(iface) = ifaces.iter_mut().find(|it| it.index == idx) {
                    iface.default = true;
                    iface.dns_servers = crate::os::unix::dns::get_system_dns_conf();
                }
            }
        }
    }

    ifaces
}
