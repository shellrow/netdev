use std::net::UdpSocket;
use std::time::{Duration, Instant};
use std::net::{IpAddr};
use pnet_packet::Packet;
use crate::interface::{self, MacAddr};

const TIMEOUT: u64 = 3000;

/// Struct of default Gateway information
#[derive(Clone, Debug)]
pub struct Gateway {
    pub mac_addr: MacAddr,
    pub ip_addr: IpAddr,
}

/// Get default Gateway
pub fn get_default_gateway() -> Result<Gateway, String> {
    let default_idx = match interface::get_default_interface_index() {
        Some(idx) => idx,
        None => return Err(String::from("Failed to get default interface")),
    };
    let interfaces = pnet_datalink::interfaces();
    let interface = interfaces.into_iter().filter(|interface: &pnet_datalink::NetworkInterface| interface.index == default_idx).next().expect("Failed to get Interface");
    let config = pnet_datalink::Config {
        write_buffer_size: 4096,
        read_buffer_size: 4096,
        read_timeout: None,
        write_timeout: None,
        channel_type: pnet_datalink::ChannelType::Layer2,
        bpf_fd_attempts: 1000,
        linux_fanout: None,
        promiscuous: false,
    };
    let (mut _tx, mut rx) = match pnet_datalink::channel(&interface, config) {
        Ok(pnet_datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unknown channel type"),
        Err(e) => panic!("Error happened {}", e),
    };
    match send_udp_packet() {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to send UDP packet {}", e)),
    }
    receive_packets(&mut rx)
}

fn send_udp_packet() -> Result<(), String> {
    let buf = [0u8; 0];
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => return Err(format!("Failed to create UDP socket {}", e)),
    };
    let dest: &str = "1.1.1.1:80";
    match socket.set_ttl(1) {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to set TTL {}", e)),
    }
    match socket.send_to(&buf, dest) {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to send data {}", e)),
    }
    Ok(())
}

fn receive_packets(rx: &mut Box<dyn pnet_datalink::DataLinkReceiver>) -> Result<Gateway, String>{
    let timeout = Duration::from_millis(TIMEOUT);
    let start_time = Instant::now();
    loop {
        match rx.next() {
            Ok(frame) => {
                let frame = match pnet_packet::ethernet::EthernetPacket::new(frame) {
                    Some(f) => f,
                    None => return Err(String::from("Failed to read packet")),
                };
                match frame.get_ethertype() {
                    pnet_packet::ethernet::EtherTypes::Ipv4 => {
                        if let Some(ip_addr) = ipv4_handler(&frame) {
                            let gateway = Gateway {
                                mac_addr: MacAddr::new(frame.get_source().octets()),
                                ip_addr: ip_addr,
                            };
                            return Ok(gateway);
                        }
                    },
                    pnet_packet::ethernet::EtherTypes::Ipv6 => {
                        if let Some(ip_addr) = ipv6_handler(&frame) {
                            let gateway = Gateway {
                                mac_addr: MacAddr::new(frame.get_source().octets()),
                                ip_addr: ip_addr,
                            };
                            return Ok(gateway);
                        }
                    },
                    _ => {}
                }
            },
            Err(e) => {
                return Err(format!("An error occurred while reading: {}", e));
            }
        }
        if Instant::now().duration_since(start_time) > timeout {
            return Err(String::from("Recieve timeout"));
        }else{
            match send_udp_packet() {
                Ok(_) => (),
                Err(e) => return Err(format!("Failed to send UDP packet {}", e)),
            }
        }
    }
}

fn ipv4_handler(ethernet: &pnet_packet::ethernet::EthernetPacket) -> Option<IpAddr> {
    if let Some(packet) = pnet_packet::ipv4::Ipv4Packet::new(ethernet.payload()) {
        match packet.get_next_level_protocol() {
            pnet_packet::ip::IpNextHeaderProtocols::Icmp => {
                return icmp_handler(&packet);
            },
            _ => {
                None
            }
        }
    }else{
        None
    }
}

fn ipv6_handler(ethernet: &pnet_packet::ethernet::EthernetPacket) -> Option<IpAddr> {
    if let Some(packet) = pnet_packet::ipv6::Ipv6Packet::new(ethernet.payload()) {
        match packet.get_next_header() {
            pnet_packet::ip::IpNextHeaderProtocols::Icmpv6 => {
                return icmpv6_handler(&packet);
            },
            _ => {
                None
            }
        }
    }else{
        None
    }
}

fn icmp_handler(ip_packet: &pnet_packet::ipv4::Ipv4Packet) -> Option<IpAddr> {
    if let Some(packet) = pnet_packet::icmp::IcmpPacket::new(ip_packet.payload()) {
        if packet.get_icmp_type() == pnet_packet::icmp::IcmpTypes::TimeExceeded {
            let ipv4_addr = ip_packet.get_source();
            return Some(IpAddr::V4(ipv4_addr))
        }else{
            None
        }
    }else{
        None
    }
}

fn icmpv6_handler(ip_packet: &pnet_packet::ipv6::Ipv6Packet) -> Option<IpAddr> {
    if let Some(packet) = pnet_packet::icmpv6::Icmpv6Packet::new(ip_packet.payload()) {
        if packet.get_icmpv6_type() == pnet_packet::icmpv6::Icmpv6Types::TimeExceeded {
            let ipv6_addr = ip_packet.get_source();
            return Some(IpAddr::V6(ipv6_addr))
        }else{
            None
        }
    }else{
        None
    }
}
