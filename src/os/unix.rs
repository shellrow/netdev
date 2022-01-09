use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use pnet_packet::Packet;
use crate::interface::{MacAddr, Interface};
use crate::gateway::Gateway;

const TIMEOUT: u64 = 3000;

fn get_default_gateway(interface_index: u32) -> Result<Gateway, String> {
    let interfaces = pnet_datalink::interfaces();
    let interface = interfaces.into_iter().filter(|interface: &pnet_datalink::NetworkInterface| interface.index == interface_index).next().expect("Failed to get Interface");
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
    let dst: &str = "1.1.1.1:80";
    match socket.set_ttl(1) {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to set TTL {}", e)),
    }
    match socket.send_to(&buf, dst) {
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

// Get network interfaces
pub fn interfaces() -> Vec<Interface> {
    let mut result: Vec<Interface> = vec![];
    let local_ip: IpAddr = match super::get_local_ipaddr(){
        Some(local_ip) => local_ip,
        None => return result,
    };
    let interfaces = pnet_datalink::interfaces();
    for iface in interfaces{
        let mac_addr: Option<MacAddr> = match iface.mac {
            Some(mac_addr) => Some(MacAddr::new(mac_addr.octets())),
            None => None,
        };
        let mut ipv4_vec: Vec<Ipv4Addr> = vec![];
        let mut ipv6_vec: Vec<Ipv6Addr> = vec![];
        let mut ips: Vec<IpAddr> = vec![];
        for ip in &iface.ips {
            match ip.ip() {
                IpAddr::V4(ipv4_addr) => {
                    ipv4_vec.push(ipv4_addr);
                },
                IpAddr::V6(ipv6_addr) => {
                    ipv6_vec.push(ipv6_addr);
                },
            }
            ips.push(ip.ip());
        }
        let default_gateway: Option<Gateway> = if ips.contains(&local_ip) {
            match get_default_gateway(iface.index) {
                Ok(default_gateway) => Some(default_gateway),
                Err(_) => None,
            }
        } else{
            None
        };
        let desc: Option<String> = if iface.description.is_empty() {
            None
        } else{
            Some(iface.description)
        };
        let interface: Interface = Interface{
            index: iface.index,
            name: iface.name,
            description: desc,
            mac_addr: mac_addr,
            ipv4: ipv4_vec,
            ipv6: ipv6_vec,
            gateway: default_gateway,
        };
        result.push(interface);
    }
    return result;
}

// Get default Interface index
pub fn default_interface_index() -> Option<u32> {
    let local_ip: IpAddr = match super::get_local_ipaddr(){
        Some(local_ip) => local_ip,
        None => return None,
    };
    let interfaces = pnet_datalink::interfaces();
    for iface in interfaces {
        for ip in iface.ips {
            if local_ip == ip.ip() {
                return Some(iface.index)
            }
        }
    }
    return None;
}

// Get default Interface name
pub fn default_interface_name() -> Option<String> {
    let local_ip: IpAddr = match super::get_local_ipaddr(){
        Some(local_ip) => local_ip,
        None => return None,
    };
    let interfaces = pnet_datalink::interfaces();
    for iface in interfaces {
        for ip in iface.ips {
            if local_ip == ip.ip() {
                return Some(iface.name)
            }
        }
    }
    return None;
}
