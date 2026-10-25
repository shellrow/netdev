use netlink_packet_route::address::{AddressAttribute, AddressMessage};
use std::net::IpAddr;

pub(crate) fn ip_from_addr(addr: &AddressMessage) -> Option<(IpAddr, u8, u32)> {
    let mut local = None;
    let mut address = None;
    let mut flags = u32::from(addr.header.flags.bits());
    for attribute in &addr.attributes {
        match attribute {
            AddressAttribute::Local(ip) => local = Some(*ip),
            AddressAttribute::Address(ip) => address = Some(*ip),
            AddressAttribute::Flags(value) => flags = value.bits(),
            _ => {}
        }
    }
    // IFA_ADDRESS is the peer address on point-to-point links. IFA_LOCAL must win
    // regardless of attribute order, while IPv6 commonly supplies only IFA_ADDRESS.
    local
        .or(address)
        .map(|ip| (ip, addr.header.prefix_len, flags))
}

#[cfg(any(target_os = "android", test))]
pub(crate) fn stats_from_link(
    link: &netlink_packet_route::link::LinkMessage,
) -> Option<crate::stats::counters::InterfaceStats> {
    use netlink_packet_route::link::LinkAttribute;
    let mut fallback = None;
    let counters = link
        .attributes
        .iter()
        .find_map(|attribute| match attribute {
            LinkAttribute::Stats64(stats) => Some((stats.rx_bytes, stats.tx_bytes)),
            LinkAttribute::Stats(stats) => {
                fallback = Some((u64::from(stats.rx_bytes), u64::from(stats.tx_bytes)));
                None
            }
            _ => None,
        })
        .or(fallback)?;
    Some(crate::stats::counters::InterfaceStats {
        rx_bytes: counters.0,
        tx_bytes: counters.1,
        timestamp: Some(std::time::SystemTime::now()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use netlink_packet_route::{
        address::AddressFlags,
        link::{LinkAttribute, LinkMessage, Stats, Stats64},
    };

    #[test]
    fn local_address_wins_in_both_attribute_orders() {
        let local = "192.0.2.1".parse().unwrap();
        let peer = "192.0.2.2".parse().unwrap();
        let mut message = AddressMessage::default();
        message.header.prefix_len = 32;
        message.attributes = vec![
            AddressAttribute::Local(local),
            AddressAttribute::Address(peer),
        ];
        assert_eq!(ip_from_addr(&message), Some((local, 32, 0)));
        message.attributes.reverse();
        assert_eq!(ip_from_addr(&message), Some((local, 32, 0)));
    }

    #[test]
    fn falls_back_to_address_and_preserves_extended_flags() {
        let ip = "2001:db8::1".parse().unwrap();
        let mut message = AddressMessage::default();
        message.header.prefix_len = 64;
        message.attributes = vec![
            AddressAttribute::Address(ip),
            AddressAttribute::Flags(AddressFlags::from_bits_retain(0x200)),
        ];
        assert_eq!(ip_from_addr(&message), Some((ip, 64, 0x200)));
        message.attributes.pop();
        assert_eq!(ip_from_addr(&message), Some((ip, 64, 0)));
        message.attributes.clear();
        assert_eq!(ip_from_addr(&message), None);
    }

    #[test]
    fn prefers_64_bit_statistics_in_both_attribute_orders() {
        let mut short = Stats::default();
        short.rx_bytes = 1;
        short.tx_bytes = 2;
        let mut wide = Stats64::default();
        wide.rx_bytes = u64::from(u32::MAX) + 2;
        wide.tx_bytes = u64::from(u32::MAX) + 3;
        let mut link = LinkMessage::default();
        link.attributes = vec![LinkAttribute::Stats(short), LinkAttribute::Stats64(wide)];
        for _ in 0..2 {
            let stats = stats_from_link(&link).unwrap();
            assert_eq!(stats.rx_bytes, u64::from(u32::MAX) + 2);
            assert_eq!(stats.tx_bytes, u64::from(u32::MAX) + 3);
            link.attributes.reverse();
        }
        link.attributes.pop();
        let stats = stats_from_link(&link).unwrap();
        assert_eq!((stats.rx_bytes, stats.tx_bytes), (1, 2));
        link.attributes.clear();
        assert_eq!(stats_from_link(&link), None);
    }
}
