use std::net::UdpSocket;
use std::time::{Duration, Instant};
use crate::socket;
use super::Gateway;

const TIMEOUT: u64 = 3000;

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

pub fn get_default_gateway(interface_name: String) -> Result<Gateway, String> {
    let timeout = Duration::from_millis(TIMEOUT);
    let start_time = Instant::now();
    let config = socket::Config {
        write_buffer_size: 4096,
        read_buffer_size: 4096,
        read_timeout: None,
        write_timeout: None,
        channel_type: socket::ChannelType::Layer2,
        bpf_fd_attempts: 1000,
        promiscuous: false,
    };
    let (mut _tx, mut rx) = match socket::channel(interface_name, config) {
        Ok(socket::Channel::Ethernet(tx, rx)) => (tx, rx),
        Err(e) => panic!("Failed to create channel {}", e),
    };
    match send_udp_packet() {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to send UDP packet {}", e)),
    }
    loop {
        match rx.next() {
            Ok(frame) => {
                match socket::packet::parse_frame(frame){
                    Ok(gateway) => {
                        return Ok(gateway);
                    },
                    Err(_) => {},
                }
            },
            Err(_) => {}
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
