#[cfg(feature = "gateway")]
use std::net::IpAddr;

use crate::{interface::interface::Interface, os::unix::interface::unix_interfaces};

pub fn interfaces() -> Vec<Interface> {
    let mut interfaces: Vec<Interface> = unix_interfaces();

    #[cfg(feature = "gateway")]
    let local_ip_opt: Option<IpAddr> = crate::net::ip::get_local_ipaddr();

    #[cfg(feature = "gateway")]
    {
        use crate::os::unix::dns::get_system_dns_conf;

        let gateway_map = super::route::get_gateway_map();

        for iface in &mut interfaces {
            if let Some(gateway) = gateway_map.get(&iface.index) {
                iface.gateway = Some(gateway.clone());
            }

            if let Some(local_ip) = local_ip_opt {
                iface.ipv4.iter().for_each(|ipv4| {
                    if IpAddr::V4(ipv4.addr()) == local_ip {
                        iface.dns_servers = get_system_dns_conf();
                        iface.default = true;
                    }
                });
                iface.ipv6.iter().for_each(|ipv6| {
                    if IpAddr::V6(ipv6.addr()) == local_ip {
                        iface.dns_servers = get_system_dns_conf();
                        iface.default = true;
                    }
                });
            }
        }
    }

    interfaces
}
