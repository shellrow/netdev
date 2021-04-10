use std::net::UdpSocket;
use pnet::transport::TransportChannelType::Layer4;
use pnet::transport::TransportProtocol::Ipv4;
//use std::time;
use std::time::{Duration, Instant};
use std::net::IpAddr;
//use pnet::packet::Packet;
use pnet::transport::icmp_packet_iter;

pub fn get_default_gateway(){
    let buf = [0u8; 0];
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => panic!("error!"),
    };
    let dest: &str = "192.168.11.1:80";
    socket.set_ttl(1).unwrap();
    socket.send_to(&buf, dest).unwrap();
    let protocol = Layer4(Ipv4(pnet::packet::ip::IpNextHeaderProtocols::Icmp));
    let (mut _tx, mut rx) = match pnet::transport::transport_channel(4096, protocol) {
        Ok((tx, rx)) => (tx, rx),
        Err(e) => panic!("Error happened {}", e),
    };
    let timeout = Duration::from_millis(3000);
    let router_ip = receive_icmp_packets(&mut rx, pnet::packet::icmp::IcmpTypes::TimeExceeded, &timeout);
    match router_ip {
        Ok(ip) => {println!("{}", ip)},
        Err(e) => {println!("{}", e)},
    }
}

#[cfg(any(unix, macos))]
pub fn receive_icmp_packets(rx: &mut pnet::transport::TransportReceiver, icmp_type: pnet::packet::icmp::IcmpType, timeout: &Duration) -> Result<String, String>{
    let mut iter = icmp_packet_iter(rx);
    let start_time = Instant::now();
    loop {
        match iter.next_with_timeout(timeout) {
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
        }
    }
}

/*
#[cfg(target_os = "windows")]
fn send_icmp_packet(){
    
}
*/
#[cfg(target_os = "windows")]
fn receive_icmp_packets(rx: &mut pnet::transport::TransportReceiver, icmp_type: pnet::packet::icmp::IcmpType, timeout: &Duration) -> Result<String, String>{
    let mut iter = icmp_packet_iter(rx);
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
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
