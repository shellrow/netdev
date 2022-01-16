use std::net::UdpSocket;
use super::Gateway;

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
    Err(String::new())
}
