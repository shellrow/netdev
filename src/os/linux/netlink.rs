use crate::os::linux::netlink_data::ip_from_addr;
use netlink_packet_core::NetlinkPayload;
use netlink_packet_route::{
    RouteNetlinkMessage,
    address::AddressMessage,
    link::{LinkAttribute, LinkMessage},
};
use netlink_sys::{Socket, protocols::NETLINK_ROUTE};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::{collections::HashMap, io};

#[cfg(feature = "gateway")]
use ipnet::{IpAdd, IpNet};
#[cfg(feature = "gateway")]
use netlink_packet_route::neighbour::{NeighbourAddress, NeighbourAttribute, NeighbourMessage};
#[cfg(feature = "gateway")]
use netlink_packet_route::route::{RouteAddress, RouteAttribute, RouteMessage};

const SEQ_BASE: u32 = 0x6E_64_65_76; // "ndev"

fn open_route_socket() -> io::Result<Socket> {
    let mut sock = Socket::new(NETLINK_ROUTE)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("netlink open: {e}")))?;
    sock.bind_auto()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bind_auto: {e}")))?;
    super::netlink_io::set_non_blocking(&sock)?;
    Ok(sock)
}

pub fn dump_links() -> io::Result<Vec<LinkMessage>> {
    let mut sock = open_route_socket()?;
    let seq = SEQ_BASE ^ 0x01;
    super::netlink_io::send_dump(
        &mut sock,
        RouteNetlinkMessage::GetLink(LinkMessage::default()),
        seq,
    )?;
    let msgs = super::netlink_io::recv_multi(&mut sock, seq)?;
    let mut out = Vec::new();
    for m in msgs {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewLink(link)) = m.payload {
            out.push(link);
        }
    }
    Ok(out)
}

pub fn dump_addrs() -> io::Result<Vec<AddressMessage>> {
    let mut sock = open_route_socket()?;
    let seq = SEQ_BASE ^ 0x02;
    super::netlink_io::send_dump(
        &mut sock,
        RouteNetlinkMessage::GetAddress(AddressMessage::default()),
        seq,
    )?;
    let msgs = super::netlink_io::recv_multi(&mut sock, seq)?;
    let mut out = Vec::new();
    for m in msgs {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewAddress(addr)) = m.payload {
            out.push(addr);
        }
    }
    Ok(out)
}

#[cfg(feature = "gateway")]
pub fn dump_routes() -> io::Result<Vec<RouteMessage>> {
    let mut sock = open_route_socket()?;
    let seq = SEQ_BASE ^ 0x03;
    super::netlink_io::send_dump(
        &mut sock,
        RouteNetlinkMessage::GetRoute(RouteMessage::default()),
        seq,
    )?;
    let msgs = super::netlink_io::recv_multi(&mut sock, seq)?;
    let mut out = Vec::new();
    for m in msgs {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewRoute(rt)) = m.payload {
            out.push(rt);
        }
    }
    Ok(out)
}

#[cfg(feature = "gateway")]
pub fn dump_neigh() -> io::Result<Vec<NeighbourMessage>> {
    let mut sock = open_route_socket()?;
    let seq = SEQ_BASE ^ 0x04;
    super::netlink_io::send_dump(
        &mut sock,
        RouteNetlinkMessage::GetNeighbour(NeighbourMessage::default()),
        seq,
    )?;
    let msgs = super::netlink_io::recv_multi(&mut sock, seq)?;
    let mut out = Vec::new();
    for m in msgs {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewNeighbour(n)) = m.payload {
            out.push(n);
        }
    }
    Ok(out)
}

fn mac_from_link(link: &LinkMessage) -> Option<[u8; 6]> {
    for nla in &link.attributes {
        if let LinkAttribute::Address(bytes) = nla {
            if bytes.len() == 6 {
                return Some([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]]);
            }
        }
    }
    None
}

fn name_from_link(link: &LinkMessage) -> Option<String> {
    for nla in &link.attributes {
        if let LinkAttribute::IfName(n) = nla {
            return Some(n.clone());
        }
    }
    None
}

#[cfg(feature = "gateway")]
fn route_addr_to_ip(a: &RouteAddress) -> Option<IpAddr> {
    match a {
        RouteAddress::Inet(v4) => Some(IpAddr::V4(*v4)),
        RouteAddress::Inet6(v6) => Some(IpAddr::V6(*v6)),
        _ => None,
    }
}

#[cfg(feature = "gateway")]
fn specific_prefix(route: &RouteMessage) -> Option<IpNet> {
    let length = route.header.destination_prefix_length;
    if length == 0 {
        return None;
    }
    route.attributes.iter().find_map(|attr| match attr {
        RouteAttribute::Destination(addr) => IpNet::new(route_addr_to_ip(addr)?, length).ok(),
        _ => None,
    })
}

#[cfg(feature = "gateway")]
fn rule_destinations() -> io::Result<Vec<IpNet>> {
    use netlink_packet_route::rule::{RuleAttribute, RuleMessage};
    let mut sock = open_route_socket()?;
    let seq = SEQ_BASE ^ 0x05;
    super::netlink_io::send_dump(
        &mut sock,
        RouteNetlinkMessage::GetRule(RuleMessage::default()),
        seq,
    )?;
    let mut prefixes = Vec::new();
    for message in super::netlink_io::recv_multi(&mut sock, seq)? {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewRule(rule)) = message.payload
            && rule.header.dst_len > 0
        {
            for attr in rule.attributes {
                if let RuleAttribute::Destination(addr) = attr
                    && let Ok(prefix) = IpNet::new(addr, rule.header.dst_len)
                {
                    prefixes.push(prefix);
                }
            }
        }
    }
    Ok(prefixes)
}

#[cfg(feature = "gateway")]
fn default_probe(prefixes: impl Iterator<Item = IpNet> + Clone, preferred: IpAddr) -> IpAddr {
    let specificity = prefixes
        .clone()
        .filter(|net| net.contains(&preferred))
        .map(|net| net.prefix_len())
        .max()
        .unwrap_or(0);
    if specificity == 0 {
        return preferred;
    }

    let mut prefixes: Vec<_> = prefixes
        .filter(|net| net.addr().is_ipv4() == preferred.is_ipv4())
        .collect();
    prefixes.sort_unstable_by_key(IpNet::network);
    // If a VPN covers the address space with two /1 routes, look outside its
    // narrower exceptions. Keep the original probe when it is equally general.
    for length in 0..specificity {
        if let Some(destination) = uncovered_destination(&prefixes, preferred, length) {
            return destination;
        }
    }
    preferred
}

#[cfg(feature = "gateway")]
fn uncovered_destination(prefixes: &[IpNet], preferred: IpAddr, min_length: u8) -> Option<IpAddr> {
    let ranges: &[(IpAddr, IpAddr)] = match preferred {
        IpAddr::V4(_) => &[
            (
                Ipv4Addr::new(1, 0, 0, 1).into(),
                Ipv4Addr::new(126, 255, 255, 255).into(),
            ),
            (
                Ipv4Addr::new(128, 0, 0, 0).into(),
                Ipv4Addr::new(223, 255, 255, 255).into(),
            ),
        ][..],
        IpAddr::V6(_) => &[(
            Ipv6Addr::from_bits(0x2000_u128 << 112).into(),
            Ipv6Addr::from_bits((0x4000_u128 << 112) - 1).into(),
        )][..],
    };

    for &(mut candidate, end) in ranges {
        for net in prefixes {
            if net.network() > candidate {
                break;
            }
            if net.prefix_len() > min_length && net.contains(&candidate) {
                candidate = match net.broadcast() {
                    IpAddr::V4(ip) => ip.saturating_add(1).into(),
                    IpAddr::V6(ip) => ip.saturating_add(1).into(),
                };
            }
        }
        if candidate <= end {
            return Some(candidate);
        }
    }
    None
}

#[cfg(feature = "gateway")]
fn neigh_addr_to_ip(a: &NeighbourAddress) -> Option<IpAddr> {
    match a {
        NeighbourAddress::Inet(v4) => Some(IpAddr::V4(*v4)),
        NeighbourAddress::Inet6(v6) => Some(IpAddr::V6(*v6)),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

#[cfg(feature = "gateway")]
fn neigh_extract(n: &NeighbourMessage) -> (Option<IpAddr>, Option<[u8; 6]>, Option<u32>) {
    let mut ip = None;
    let mut mac = None;
    let mut oif = Some(n.header.ifindex as u32);

    for nla in &n.attributes {
        match nla {
            NeighbourAttribute::Destination(a) => {
                ip = neigh_addr_to_ip(a);
            }
            // Link-layer address (MAC)
            NeighbourAttribute::LinkLayerAddress(v) => {
                if v.len() == 6 {
                    mac = Some([v[0], v[1], v[2], v[3], v[4], v[5]]);
                }
            }
            NeighbourAttribute::IfIndex(i) => oif = Some(*i as u32),
            _ => {}
        }
    }
    (ip, mac, oif)
}

fn mtu_from_link(link: &LinkMessage) -> Option<u32> {
    for nla in &link.attributes {
        if let LinkAttribute::Mtu(m) = nla {
            return Some(*m);
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct IfRow {
    pub index: u32,
    pub name: String,
    pub mac: Option<[u8; 6]>,
    pub ipv4: Vec<(Ipv4Addr, u8)>,
    pub ipv6: Vec<(Ipv6Addr, u8)>,
    pub ipv6_addr_flags: Vec<u32>,
    pub flags: u32,
    pub mtu: Option<u32>,
    pub stats: Option<crate::stats::counters::InterfaceStats>,
}

pub fn collect_interfaces() -> io::Result<Vec<IfRow>> {
    let links = dump_links()?;
    let addrs = dump_addrs()?;

    let mut base: HashMap<u32, IfRow> = HashMap::new();
    for l in links {
        let idx = l.header.index as u32;
        let name = name_from_link(&l).unwrap_or_else(|| idx.to_string());
        let mac = mac_from_link(&l);
        let flags = l.header.flags.bits();
        let mtu_nl = mtu_from_link(&l);
        base.insert(
            idx,
            IfRow {
                index: idx,
                name,
                mac,
                ipv4: vec![],
                ipv6: vec![],
                ipv6_addr_flags: vec![],
                flags,
                mtu: mtu_nl,
                stats: super::netlink_data::stats_from_link(&l),
            },
        );
    }

    for a in addrs {
        let idx = a.header.index as u32;
        if let Some((ip, pfx, addr_flags)) = ip_from_addr(&a) {
            if let Some(row) = base.get_mut(&idx) {
                match ip {
                    IpAddr::V4(v4) => row.ipv4.push((v4, pfx)),
                    IpAddr::V6(v6) => {
                        row.ipv6.push((v6, pfx));
                        row.ipv6_addr_flags.push(addr_flags);
                    }
                }
            }
        }
    }

    Ok(base.into_values().collect())
}

#[cfg(feature = "gateway")]
#[derive(Debug, Clone)]
pub struct GwRow {
    #[allow(dead_code)]
    pub ifindex: u32,
    pub gw_v4: Vec<Ipv4Addr>,
    pub gw_v6: Vec<Ipv6Addr>,
    pub mac: Option<[u8; 6]>,
}

#[cfg(feature = "gateway")]
pub struct RouteSnapshot {
    pub gateways: HashMap<u32, GwRow>,
    pub default_source: Option<IpAddr>,
}

#[cfg(feature = "gateway")]
pub fn collect_routes() -> io::Result<RouteSnapshot> {
    use crate::net::ip::{IPV4_PROBE, IPV6_PROBE, source_address_for};
    let routes = dump_routes()?;
    // A destination-specific rule can select a VPN table containing only a default route.
    let rules = rule_destinations().unwrap_or_default();
    let prefixes = routes
        .iter()
        .filter_map(specific_prefix)
        .chain(rules.iter().copied());
    let default_source = source_address_for(default_probe(prefixes.clone(), IPV4_PROBE))
        .or_else(|| source_address_for(default_probe(prefixes, IPV6_PROBE)));
    let neighs = dump_neigh().unwrap_or_default();

    let mut m: HashMap<u32, GwRow> = HashMap::new();
    for rt in routes {
        if rt.header.destination_prefix_length != 0 {
            continue;
        }
        let mut gw = None;
        let mut oif = None;
        for attr in &rt.attributes {
            match attr {
                RouteAttribute::Gateway(addr) => gw = route_addr_to_ip(addr),
                RouteAttribute::Oif(index) => oif = Some(*index),
                _ => {}
            }
        }
        let Some(oif) = oif else { continue };
        if let Some(gwip) = gw {
            let e = m.entry(oif).or_insert(GwRow {
                ifindex: oif,
                gw_v4: vec![],
                gw_v6: vec![],
                mac: None,
            });
            match gwip {
                IpAddr::V4(v4) => {
                    if !e.gw_v4.contains(&v4) {
                        e.gw_v4.push(v4);
                    }
                }
                IpAddr::V6(v6) => {
                    if !e.gw_v6.contains(&v6) {
                        e.gw_v6.push(v6);
                    }
                }
            }
        }
    }

    let mut mac_candidates = super::gateway::GatewayMacCandidates::default();
    for n in neighs {
        let (ip, mac, ifi) = neigh_extract(&n);
        let ifi = match ifi {
            Some(i) => i,
            None => continue,
        };
        let (Some(row), Some(m6), Some(ip)) = (m.get_mut(&ifi), mac, ip) else {
            continue;
        };
        match ip {
            IpAddr::V4(v4) if row.gw_v4.contains(&v4) => {
                mac_candidates.record_ipv4(ifi, m6);
            }
            IpAddr::V6(v6) if row.gw_v6.contains(&v6) => {
                mac_candidates.record_ipv6(ifi, m6);
            }
            _ => {}
        }
    }

    for (ifindex, row) in &mut m {
        // Preserve the established IPv4 result when both families use different routers.
        row.mac = mac_candidates.get(*ifindex);
    }

    Ok(RouteSnapshot {
        gateways: m,
        default_source,
    })
}

#[cfg(all(test, feature = "gateway"))]
mod tests {
    use super::*;
    use crate::net::ip::{IPV4_PROBE, IPV6_PROBE};

    fn route(cidr: &str) -> RouteMessage {
        let prefix: IpNet = cidr.parse().unwrap();
        let mut route = RouteMessage::default();
        route.header.destination_prefix_length = prefix.prefix_len();
        route
            .attributes
            .push(RouteAttribute::Destination(match prefix.addr() {
                IpAddr::V4(addr) => RouteAddress::Inet(addr),
                IpAddr::V6(addr) => RouteAddress::Inet6(addr),
            }));
        route
    }

    #[test]
    fn split_routes_do_not_select_the_default_interface() {
        for (probe, defaults, split) in [
            (
                IPV4_PROBE,
                "0.0.0.0/0",
                ["10.0.0.0/8", "1.0.0.0/8", "2.0.0.0/7"],
            ),
            (IPV6_PROBE, "::/0", ["fc00::/7", "2000::/16", "2001::/16"]),
        ] {
            let mut routes = vec![route(defaults)];
            assert_eq!(
                default_probe(routes.iter().filter_map(specific_prefix), probe),
                probe
            );
            routes.extend(split.map(route));
            let selected = default_probe(routes.iter().filter_map(specific_prefix), probe);
            assert_eq!(probe.is_ipv4(), selected.is_ipv4());
            assert!(
                routes
                    .iter()
                    .filter_map(specific_prefix)
                    .all(|net| !net.contains(&selected))
            );
        }
    }

    #[test]
    fn covering_routes_preserve_the_kernel_lookup() {
        let mut routes = vec![route("0.0.0.0/1"), route("128.0.0.0/1")];
        assert_eq!(
            default_probe(routes.iter().filter_map(specific_prefix), IPV4_PROBE),
            IPV4_PROBE
        );
        routes.push(route("10.0.0.0/8"));
        let selected = default_probe(routes.iter().filter_map(specific_prefix), IPV4_PROBE);
        assert!(!specific_prefix(&routes[2]).unwrap().contains(&selected));
    }
}
