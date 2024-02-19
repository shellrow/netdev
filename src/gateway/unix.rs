use crate::device::NetworkDevice;
use crate::socket;
use std::time::{Duration, Instant};

const TIMEOUT: u64 = 3000;

pub fn get_default_gateway(interface_name: String) -> Result<NetworkDevice, String> {
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
    let (mut _tx, mut rx);
    match socket::channel(interface_name, config) {
        Ok(socket::Channel::Ethernet(etx, erx)) => {
            _tx = etx;
            rx = erx;
        }
        Err(e) => return Err(format!("Failed to create channel {}", e)),
    }
    match super::send_udp_packet() {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to send UDP packet {}", e)),
    }
    loop {
        match rx.next() {
            Ok(frame) => match socket::packet::parse_frame(frame) {
                Ok(gateway) => {
                    return Ok(gateway);
                }
                Err(_) => {}
            },
            Err(_) => {}
        }
        if Instant::now().duration_since(start_time) > timeout {
            return Err(String::from("Recieve timeout"));
        } else {
            match super::send_udp_packet() {
                Ok(_) => (),
                Err(e) => return Err(format!("Failed to send UDP packet {}", e)),
            }
        }
    }
}
