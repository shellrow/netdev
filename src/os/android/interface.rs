use super::netlink;
use crate::interface::interface::Interface;
use crate::interface::state::OperState;
use crate::ipnet::{Ipv4Net, Ipv6Net};
use crate::net::mac::MacAddr;
use std::net::{Ipv4Addr, Ipv6Addr};

#[cfg(feature = "gateway")]
use crate::net::device::NetworkDevice;
#[cfg(feature = "gateway")]
use crate::os::unix::dns::get_system_dns_conf;
#[cfg(feature = "gateway")]
use std::collections::HashMap;

use crate::os::unix::interface::unix_interfaces;

fn push_ipv4(v: &mut Vec<Ipv4Net>, add: (Ipv4Addr, u8)) {
    if v.iter()
        .any(|n| n.addr() == add.0 && n.prefix_len() == add.1)
    {
        return;
    }
    if let Ok(net) = Ipv4Net::new(add.0, add.1) {
        v.push(net);
    }
}

fn push_ipv6(v: &mut Vec<Ipv6Net>, add: (Ipv6Addr, u8)) {
    if v.iter()
        .any(|n| n.addr() == add.0 && n.prefix_len() == add.1)
    {
        return;
    }
    if let Ok(net) = Ipv6Net::new(add.0, add.1) {
        v.push(net);
    }
}

#[inline]
fn calc_v6_scope_id(addr: &Ipv6Addr, ifindex: u32) -> u32 {
    let seg0 = addr.segments()[0];
    if (seg0 & 0xffc0) == 0xfe80 {
        ifindex
    } else {
        0
    }
}

pub fn interfaces() -> Vec<Interface> {
    let mut ifaces: Vec<Interface> = Vec::new();

    match netlink::collect_interfaces() {
        Ok(rows) => {
            for r in rows {
                let name = r.name.clone();
                let mut iface = Interface {
                    index: r.index,
                    name: name.clone(),
                    friendly_name: None,
                    description: None,
                    if_type: super::types::guess_type_by_name(&name).unwrap_or(r.if_type),
                    mac_addr: r.mac.map(MacAddr::from_octets),
                    ipv4: Vec::new(),
                    ipv6: Vec::new(),
                    ipv6_scope_ids: Vec::new(),
                    flags: r.flags,
                    oper_state: OperState::from_if_flags(r.flags),
                    transmit_speed: None,
                    receive_speed: None,
                    stats: r.stats.clone(),
                    #[cfg(feature = "gateway")]
                    gateway: None,
                    #[cfg(feature = "gateway")]
                    dns_servers: Vec::new(),
                    mtu: r.mtu,
                    #[cfg(feature = "gateway")]
                    default: false,
                };

                for (a, p) in r.ipv4 {
                    push_ipv4(&mut iface.ipv4, (a, p));
                }
                for (a, p) in r.ipv6 {
                    push_ipv6(&mut iface.ipv6, (a, p));
                    iface.ipv6_scope_ids.push(calc_v6_scope_id(&a, iface.index));
                }

                ifaces.push(iface);
            }
        }
        Err(_) => {
            // fallback: unix ifaddrs
            ifaces = unix_interfaces();

            for iface in &mut ifaces {
                if let Some(t) = super::types::guess_type_by_name(&iface.name) {
                    iface.if_type = t;
                }
            }
        }
    }

    // Fill gateway info
    #[cfg(feature = "gateway")]
    {
        if let Ok(gmap) = netlink::collect_routes() {
            let by_index: HashMap<u32, &netlink::GwRow> =
                gmap.iter().map(|(k, v)| (*k, v)).collect();

            for iface in &mut ifaces {
                if iface.index == 0 {
                    continue;
                }
                if let Some(row) = by_index.get(&iface.index) {
                    let dev = NetworkDevice {
                        mac_addr: row.mac.map(MacAddr::from_octets).unwrap_or(MacAddr::zero()),
                        ipv4: row.gw_v4.clone(),
                        ipv6: row.gw_v6.clone(),
                    };
                    iface.gateway = Some(dev);
                }
            }
        }

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
