use std::convert::TryInto;
use crate::gateway::Gateway;
use crate::interface::MacAddr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::u16;

pub const ETHER_TYPE_IPV4: [u8; 2] = [8, 0];
pub const ETHER_TYPE_IPV6: [u8; 2] = [134, 221];
pub const NEXT_HEADER_ICMP: u8 = 1;
pub const NEXT_HEADER_ICMPV6: u8 = 58;
pub const ICMP_TYPE_TIME_EXCEEDED: u8 = 11;
pub const ICMPV6_TYPE_TIME_EXCEEDED: u8 = 3;

#[allow(dead_code)]
pub enum Frame {
    SrcMacAddr,
    DstMacAddr,
    EtherType,
    SrcIpv4Addr,
    DstIpv4Addr,
    SrcIpv6Addr,
    DstIpv6Addr,
    NextHeaderProtocolIpv4,
    NextHeaderProtocolIpv6,
    IcmpType,
    Icmpv6Type,
}

impl Frame {
    fn start_index(&self) -> usize {
        match *self {
            Frame::SrcMacAddr => 6,
            Frame::DstMacAddr => 0,
            Frame::EtherType => 12,
            Frame::SrcIpv4Addr => 26,
            Frame::DstIpv4Addr => 30,
            Frame::SrcIpv6Addr => 22,
            Frame::DstIpv6Addr => 38,
            Frame::NextHeaderProtocolIpv4 => 23,
            Frame::NextHeaderProtocolIpv6 => 20,
            Frame::IcmpType => 34,
            Frame::Icmpv6Type => 54,
        }
    }
    fn end_index(&self) -> usize {
        match *self {
            Frame::SrcMacAddr => 12,
            Frame::DstMacAddr => 6,
            Frame::EtherType => 14,
            Frame::SrcIpv4Addr => 30,
            Frame::DstIpv4Addr => 34,
            Frame::SrcIpv6Addr => 38,
            Frame::DstIpv6Addr => 54,
            Frame::NextHeaderProtocolIpv4 => 24,
            Frame::NextHeaderProtocolIpv6 => 21,
            Frame::IcmpType => 35,
            Frame::Icmpv6Type => 55,
        }
    }
}

fn convert_ipv4_bytes(bytes: [u8; 4]) -> Ipv4Addr {
    Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3])
}

fn convert_ipv6_bytes(bytes: [u8; 16]) -> Ipv6Addr {
    let h1: u16 = ((bytes[0] as u16) << 8) | bytes[1] as u16;
    let h2: u16 = ((bytes[2] as u16) << 8) | bytes[3] as u16;
    let h3: u16 = ((bytes[4] as u16) << 8) | bytes[5] as u16;
    let h4: u16 = ((bytes[6] as u16) << 8) | bytes[7] as u16;
    let h5: u16 = ((bytes[8] as u16) << 8) | bytes[9] as u16;
    let h6: u16 = ((bytes[10] as u16) << 8) | bytes[11] as u16;
    let h7: u16 = ((bytes[12] as u16) << 8) | bytes[13] as u16;
    let h8: u16 = ((bytes[14] as u16) << 8) | bytes[15] as u16;
    Ipv6Addr::new(h1, h2, h3, h4, h5, h6, h7, h8)
}

pub fn parse_frame(frame: &[u8]) -> Result<Gateway, ()> {
    let src_mac: [u8; 6] = frame[Frame::SrcMacAddr.start_index()..Frame::SrcMacAddr.end_index()].try_into().unwrap();
    let ether_type: [u8; 2] = frame[Frame::EtherType.start_index()..Frame::EtherType.end_index()].try_into().unwrap();
    match ether_type {
        ETHER_TYPE_IPV4 => {
            let src_ip: [u8; 4] = frame[Frame::SrcIpv4Addr.start_index()..Frame::SrcIpv4Addr.end_index()].try_into().unwrap();
            let next_header_protocol: u8 = frame[Frame::NextHeaderProtocolIpv4.start_index()];
            if next_header_protocol == NEXT_HEADER_ICMP {
                let icmp_type: u8 = frame[Frame::IcmpType.start_index()];
                if icmp_type == ICMP_TYPE_TIME_EXCEEDED {
                    let gateway = Gateway {
                        mac_addr: MacAddr::new(src_mac),
                        ip_addr: IpAddr::V4(convert_ipv4_bytes(src_ip)),
                    };
                    return Ok(gateway);
                }
            }
        },
        ETHER_TYPE_IPV6 => {
            let src_ip: [u8; 16] = frame[Frame::SrcIpv6Addr.start_index()..Frame::SrcIpv6Addr.end_index()].try_into().unwrap();
            let next_header_protocol: u8 = frame[Frame::NextHeaderProtocolIpv6.start_index()];
            if next_header_protocol == NEXT_HEADER_ICMPV6 {
                let icmp_type: u8 = frame[Frame::Icmpv6Type.start_index()];
                if icmp_type == ICMPV6_TYPE_TIME_EXCEEDED {
                    let icmp_type: u8 = frame[Frame::Icmpv6Type.start_index()];
                    if icmp_type == ICMPV6_TYPE_TIME_EXCEEDED {
                        let gateway = Gateway {
                            mac_addr: MacAddr::new(src_mac),
                            ip_addr: IpAddr::V6(convert_ipv6_bytes(src_ip)),
                        };
                        return Ok(gateway);
                    }
                }
            }
        },
        _ => {},
    }
    Err(())
}
