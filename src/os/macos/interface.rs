use crate::{
    interface::{interface::Interface, types::InterfaceType},
    os::unix::interface::unix_interfaces,
};

#[derive(Debug)]
pub struct SCInterface {
    #[allow(dead_code)]
    pub name: String,
    pub friendly_name: Option<String>,
    pub interface_type: InterfaceType,
}

pub fn interfaces() -> Vec<Interface> {
    let type_map = super::types::get_if_type_map();
    let mut ifaces: Vec<Interface> = unix_interfaces();

    #[cfg(feature = "gateway")]
    let gateway_map = crate::os::darwin::route::get_gateway_map();

    for iface in &mut ifaces {
        if let Some(sc_interface) = type_map.get(&iface.name) {
            iface.if_type = sc_interface.interface_type;
            iface.friendly_name = sc_interface.friendly_name.clone();
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
        use crate::os::unix::dns::get_system_dns_conf;
        if let Some(local_ip) = crate::net::ip::get_local_ipaddr() {
            if let Some(idx) = crate::interface::pick_default_iface_index(&ifaces, local_ip) {
                if let Some(iface) = ifaces.iter_mut().find(|it| it.index == idx) {
                    iface.default = true;
                    iface.dns_servers = get_system_dns_conf();
                }
            }
        }
    }

    ifaces
}
