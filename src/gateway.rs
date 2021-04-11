use std::net::UdpSocket;
use std::time::{Duration, Instant};
use std::net::{IpAddr, Ipv4Addr};
use pnet::packet::Packet;
use pnet::packet::MutablePacket;

#[cfg(any(unix, macos))]
use pnet::transport::TransportChannelType::Layer4;
#[cfg(any(unix, macos))]
use pnet::transport::TransportProtocol::Ipv4;
#[cfg(any(unix, macos))]
use pnet::transport::icmp_packet_iter;

#[cfg(target_os = "windows")]
use pnet::datalink;

use crate::interface;

/// Struct of default Gateway information
pub struct Gateway {
    pub ip: Option<String>,
    pub mac: Option<String>,
}

/// Get default Gateway
pub fn get_default_gateway() -> Gateway {
    let mut gateway: Gateway = Gateway {
        ip: None,
        mac: None,
    };
    let default_gateway_ip: Option<String> = match get_default_gateway_ip(){
        Ok(gateway_ip) => Some(gateway_ip),
        Err(_) => None,
    };
    gateway.ip = default_gateway_ip.clone();
    if let Some(gateway_ip) = default_gateway_ip.clone(){
        let default_gateway_mac: Option<String> = match get_default_gateway_mac(gateway_ip.to_string()) {
            Ok(gateway_mac) => Some(gateway_mac),
            Err(_) => None,
        };
        gateway.mac = default_gateway_mac;
    }
    return gateway;
}

/// Get default Gateway IP address
pub fn get_default_gateway_ip() -> Result<String,String>{
    send_udp_packet();
    let timeout = Duration::from_millis(3000);
    let r = receive_icmp_packets(pnet::packet::icmp::IcmpTypes::TimeExceeded, &timeout);
    return r;
}

/// Get default Gateway MAC address
pub fn get_default_gateway_mac(gateway_ip: String) -> Result<String,String>{
    match gateway_ip.parse::<Ipv4Addr>(){
        Ok(ipv4_addr) => {
            if let Some(gateway_mac) = get_mac_through_arp(ipv4_addr){
                return Ok(gateway_mac);
            }else{
                return Err(String::from("Failed to get gateway mac address"));
            }
        },
        Err(_) => return Err(String::from("Failed to get gateway mac address")),
    }
}

fn send_udp_packet(){
    let buf = [0u8; 0];
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => panic!("Error happened {}", e),
    };
    let dest: &str = "8.8.8.8:80";
    socket.set_ttl(1).unwrap();
    socket.send_to(&buf, dest).unwrap();
}

#[cfg(any(unix, macos))]
fn receive_icmp_packets(icmp_type: pnet::packet::icmp::IcmpType, timeout: &Duration) -> Result<String, String>{
    let protocol = Layer4(Ipv4(pnet::packet::ip::IpNextHeaderProtocols::Icmp));
    let (mut _tx, mut rx) = match pnet::transport::transport_channel(4096, protocol) {
        Ok((tx, rx)) => (tx, rx),
        Err(e) => panic!("Error happened {}", e),
    };
    let mut iter = icmp_packet_iter(&mut rx);
    let start_time = Instant::now();
    loop {
        match iter.next_with_timeout(*timeout) {
            Ok(r) => {
                if let Some((packet, addr)) = r {
                    if packet.get_icmp_type() == icmp_type {
                        match addr {
                            IpAddr::V4(ipv4_addr) =>{return Ok(ipv4_addr.to_string())},
                            IpAddr::V6(ipv6_addr) =>{return Ok(ipv6_addr.to_string())},
                        }
                    }
                }else{
                    return Err(String::from("Failed to read packet"));
                }
            },
            Err(e) => {
                return Err(format!("An error occurred while reading: {}", e));
            }
        }
        if Instant::now().duration_since(start_time) > *timeout {
            return Err(String::from("timeout"));
        }else{
            send_udp_packet();
        }
    }
}

#[cfg(target_os = "windows")]
fn receive_icmp_packets(icmp_type: pnet::packet::icmp::IcmpType, timeout: &Duration) -> Result<String, String>{
    /*
    let protocol = Layer4(Ipv4(pnet::packet::ip::IpNextHeaderProtocols::Icmp));
    let (mut _tx, mut rx) = match pnet::transport::transport_channel(4096, protocol) {
        Ok((tx, rx)) => (tx, rx),
        Err(e) => panic!("Error happened {}", e),
    };
    let mut iter = icmp_packet_iter(&mut rx);
    let start_time = Instant::now();
    loop {
        match iter.next() {
            Ok((packet, addr)) => {
                if packet.get_icmp_type() == icmp_type {
                    match addr {
                        IpAddr::V4(ipv4_addr) =>{return Ok(ipv4_addr.to_string())},
                        IpAddr::V6(ipv6_addr) =>{return Ok(ipv6_addr.to_string())},
                    }
                }
            },
            Err(e) => {
                return Err(format!("An error occurred while reading: {}", e));
            }
        }
        if Instant::now().duration_since(start_time) > *timeout {
            return Err(String::from("timeout"));
        }else{
            send_udp_packet();
        }
    }
    */
    let default_idx = interface::get_default_interface_index().unwrap();
    let interfaces = pnet::datalink::interfaces();
    let interface = interfaces.into_iter().filter(|interface: &pnet::datalink::NetworkInterface| interface.index == default_idx).next().expect("Failed to get Interface");
    let (mut _tx, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unknown channel type"),
        Err(e) => panic!("Error happened {}", e),
    };
    receive_packets(&mut rx, icmp_type, timeout)
}

#[cfg(target_os = "windows")]
fn receive_packets(rx: &mut Box<dyn pnet::datalink::DataLinkReceiver>, icmp_type: pnet::packet::icmp::IcmpType, timeout: &Duration) -> Result<String, String>{
    let start_time = Instant::now();
    loop {
        match rx.next() {
            Ok(frame) => {
                let frame = pnet::packet::ethernet::EthernetPacket::new(frame).unwrap();
                match frame.get_ethertype() {
                    pnet::packet::ethernet::EtherTypes::Ipv4 => {
                        if let Some(ip_addr) = ipv4_handler(&frame, icmp_type){
                            return Ok(ip_addr);
                        }
                    },
                    pnet::packet::ethernet::EtherTypes::Ipv6 => {
                        if let Some(ip_addr) = ipv6_handler(&frame, icmp_type){
                            return Ok(ip_addr);
                        }
                    },
                    _ => {
                        //println!("Not a ipv4 or ipv6");
                    }
                }
            },
            Err(e) => {
                return Err(format!("An error occurred while reading: {}", e));
            }
        }
        if Instant::now().duration_since(start_time) > *timeout {
            return Err(String::from("timeout"));
        }else{
            send_udp_packet();
        }
    }
}

#[cfg(target_os = "windows")]
fn ipv4_handler(ethernet: &pnet::packet::ethernet::EthernetPacket, icmp_type: pnet::packet::icmp::IcmpType) -> Option<String> {
    if let Some(packet) = pnet::packet::ipv4::Ipv4Packet::new(ethernet.payload()){
        match packet.get_next_level_protocol() {
            pnet::packet::ip::IpNextHeaderProtocols::Icmp => {
                return icmp_handler(&packet, icmp_type);
            },
            _ => {
                None
            }
        }
    }else{
        None
    }
}

#[cfg(target_os = "windows")]
fn ipv6_handler(ethernet: &pnet::packet::ethernet::EthernetPacket, icmp_type: pnet::packet::icmp::IcmpType) -> Option<String> {
    if let Some(packet) = pnet::packet::ipv6::Ipv6Packet::new(ethernet.payload()){
        match packet.get_next_header() {
            pnet::packet::ip::IpNextHeaderProtocols::Icmpv6 => {
                return icmpv6_handler(&packet, icmp_type);
            },
            _ => {
                None
            }
        }
    }else{
        None
    }
}

#[cfg(target_os = "windows")]
fn icmp_handler(ip_packet: &pnet::packet::ipv4::Ipv4Packet, icmp_type: pnet::packet::icmp::IcmpType) -> Option<String> {
    if let Some(packet) = pnet::packet::icmp::IcmpPacket::new(ip_packet.payload()){
        if packet.get_icmp_type() == icmp_type {
            let ipv4_addr = ip_packet.get_source();
            return Some(ipv4_addr.to_string())
        }else{
            None
        }
    }else{
        None
    }
}

#[cfg(target_os = "windows")]
fn icmpv6_handler(ip_packet: &pnet::packet::ipv6::Ipv6Packet, icmp_type: pnet::packet::icmp::IcmpType) -> Option<String> {
    if let Some(packet) = pnet::packet::icmp::IcmpPacket::new(ip_packet.payload()){
        if packet.get_icmp_type() == icmp_type {
            let ipv6_addr = ip_packet.get_source();
            return Some(ipv6_addr.to_string())
        }else{
            None
        }
    }else{
        None
    }
}

fn get_mac_through_arp(dst_ip: Ipv4Addr) -> Option<String> {
    let default_idx = interface::get_default_interface_index().unwrap();
    let interfaces = pnet::datalink::interfaces();
    let interface = interfaces.into_iter().filter(|interface: &pnet::datalink::NetworkInterface| interface.index == default_idx).next().expect("Failed to get Interface");
    let src_ip = interface.ips.iter().find(|ip| ip.is_ipv4())
        .map(|ip| match ip.ip() {
            IpAddr::V4(ip) => ip,
            _ => unreachable!(),
        })
        .unwrap();
    let (mut sender, mut receiver) = match pnet::datalink::channel(&interface, Default::default()) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unknown channel type"),
        Err(e) => panic!("Error happened {}", e),
    };

    let mut ethernet_buffer = [0u8; 42];
    let mut ethernet_packet = pnet::packet::ethernet::MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

    ethernet_packet.set_destination(pnet::datalink::MacAddr::broadcast());
    ethernet_packet.set_source(interface.mac.unwrap());
    ethernet_packet.set_ethertype(pnet::packet::ethernet::EtherTypes::Arp);

    let mut arp_buffer = [0u8; 28];
    let mut arp_packet = pnet::packet::arp::MutableArpPacket::new(&mut arp_buffer).unwrap();

    arp_packet.set_hardware_type(pnet::packet::arp::ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(pnet::packet::ethernet::EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(pnet::packet::arp::ArpOperations::Request);
    arp_packet.set_sender_hw_addr(interface.mac.unwrap());
    arp_packet.set_sender_proto_addr(src_ip);
    arp_packet.set_target_hw_addr(pnet::datalink::MacAddr::zero());
    arp_packet.set_target_proto_addr(dst_ip);

    ethernet_packet.set_payload(arp_packet.packet_mut());

    sender.send_to(ethernet_packet.packet(), None).unwrap().unwrap();

    let mut target_mac_addr: pnet::datalink::MacAddr = pnet::datalink::MacAddr::zero();

    for _x in 0..2 {
        let buf = receiver.next().unwrap();
        let arp = pnet::packet::arp::ArpPacket::new(&buf[pnet::packet::ethernet::MutableEthernetPacket::minimum_packet_size()..]).unwrap();
        if arp.get_sender_hw_addr() != interface.mac.unwrap() {
            target_mac_addr = arp.get_sender_hw_addr();
        }
    }
    if target_mac_addr == pnet::datalink::MacAddr::zero() {
        return None;
    }else{
        return Some(target_mac_addr.to_string());
    }
}
