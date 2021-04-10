use std::net::UdpSocket;
use pnet::transport::TransportChannelType::Layer4;
use pnet::transport::TransportProtocol::Ipv4;
use std::time::{Duration, Instant};
use std::net::IpAddr;
use pnet::transport::icmp_packet_iter;
use pnet::datalink;

#[cfg(target_os = "windows")]
use pnet::packet::Packet;

pub fn get_default_gateway() -> Result<String,String> {
    send_udp_packet();
    let timeout = Duration::from_millis(3000);
    let r = receive_icmp_packets(pnet::packet::icmp::IcmpTypes::TimeExceeded, &timeout);
    return r;
}

pub fn send_udp_packet(){
    let buf = [0u8; 0];
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => panic!("error!"),
    };
    let dest: &str = "8.8.8.8:80";
    socket.set_ttl(1).unwrap();
    socket.send_to(&buf, dest).unwrap();
}

#[cfg(any(unix, macos))]
pub fn receive_icmp_packets(icmp_type: pnet::packet::icmp::IcmpType, timeout: &Duration) -> Result<String, String>{
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
    let default_idx = get_default_interface_index().unwrap();
    let interfaces = pnet::datalink::interfaces();
    let interface = interfaces.into_iter().filter(|interface: &pnet::datalink::NetworkInterface| interface.index == default_idx).next().expect("Failed to get Interface");
    let (mut _tx, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unknown channel type"),
        Err(e) => panic!("Error happened {}", e),
    };
    receive_packets(&mut rx, timeout)
}

#[cfg(target_os = "windows")]
fn receive_packets(rx: &mut Box<dyn pnet::datalink::DataLinkReceiver>, timeout: &Duration) -> Result<String, String>{
    let start_time = Instant::now();
    loop {
        match rx.next() {
            Ok(frame) => {
                let frame = pnet::packet::ethernet::EthernetPacket::new(frame).unwrap();
                match frame.get_ethertype() {
                    pnet::packet::ethernet::EtherTypes::Ipv4 => {
                        if let Some(ip_addr) = ipv4_handler(&frame){
                            return Ok(ip_addr);
                        }
                    },
                    pnet::packet::ethernet::EtherTypes::Ipv6 => {
                        if let Some(ip_addr) = ipv6_handler(&frame){
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
fn ipv4_handler(ethernet: &pnet::packet::ethernet::EthernetPacket) -> Option<String> {
    if let Some(packet) = pnet::packet::ipv4::Ipv4Packet::new(ethernet.payload()){
        match packet.get_next_level_protocol() {
            pnet::packet::ip::IpNextHeaderProtocols::Icmp => {
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

#[cfg(target_os = "windows")]
fn ipv6_handler(ethernet: &pnet::packet::ethernet::EthernetPacket) -> Option<String> {
    if let Some(packet) = pnet::packet::ipv6::Ipv6Packet::new(ethernet.payload()){
        match packet.get_next_header() {
            pnet::packet::ip::IpNextHeaderProtocols::Icmpv6 => {
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

#[cfg(target_os = "windows")]
fn icmp_handler(ip_packet: &pnet::packet::ipv4::Ipv4Packet) -> Option<String> {
    if let Some(packet) = pnet::packet::icmp::IcmpPacket::new(ip_packet.payload()){
        if packet.get_icmp_type() == pnet::packet::icmp::IcmpTypes::TimeExceeded {
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
fn icmpv6_handler(ip_packet: &pnet::packet::ipv6::Ipv6Packet) -> Option<String> {
    if let Some(packet) = pnet::packet::icmp::IcmpPacket::new(ip_packet.payload()){
        if packet.get_icmp_type() == pnet::packet::icmp::IcmpTypes::TimeExceeded {
            let ipv6_addr = ip_packet.get_source();
            return Some(ipv6_addr.to_string())
        }else{
            None
        }
    }else{
        None
    }
}

pub fn get_local_ipaddr() -> Option<String> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    match socket.connect("8.8.8.8:80") {
        Ok(()) => (),
        Err(_) => return None,
    };

    match socket.local_addr() {
        Ok(addr) => return Some(addr.ip().to_string()),
        Err(_) => return None,
    };
}

pub fn get_default_interface_index() -> Option<u32> {
    let local_ip = get_local_ipaddr();
    let all_interfaces = datalink::interfaces();
    if let Some(local_ip) = local_ip {
        for iface in all_interfaces{
            for ip in iface.ips{
                match ip.ip() {
                    IpAddr::V4(ipv4) => {
                        if local_ip == ipv4.to_string() {
                            return Some(iface.index)
                        }
                    },
                    IpAddr::V6(ipv6) => {
                        if local_ip == ipv6.to_string() {
                            return Some(iface.index)
                        }
                    },
                }
            }
        }
        return None;
    }else{
        return None;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
