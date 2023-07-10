#[cfg(any(
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
pub mod packet;

#[cfg(any(
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
mod unix;
#[cfg(any(
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
pub use self::unix::*;

#[cfg(any(
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;
    fn send_udp_packet() {
        let buf = [0u8; 0];
        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(e) => {
                println!("Failed to create UDP socket {}", e);
                return;
            }
        };
        let dst: &str = "1.1.1.1:80";
        match socket.set_ttl(1) {
            Ok(_) => (),
            Err(e) => {
                println!("Failed to set TTL {}", e);
                return;
            }
        }
        match socket.send_to(&buf, dst) {
            Ok(_) => (),
            Err(e) => {
                println!("Failed to send data {}", e);
                return;
            }
        }
    }
    #[test]
    fn test_packet_capture() {
        let interface_name: String = String::from("en7");
        let config = Config {
            write_buffer_size: 4096,
            read_buffer_size: 4096,
            read_timeout: None,
            write_timeout: None,
            channel_type: ChannelType::Layer2,
            bpf_fd_attempts: 1000,
            promiscuous: false,
        };
        let (mut _tx, mut rx) = match channel(interface_name, config) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Err(e) => panic!("Error happened {}", e),
        };

        send_udp_packet();

        loop {
            match rx.next() {
                Ok(frame) => match packet::parse_frame(frame) {
                    Ok(gateway) => {
                        println!("Default Gateway:");
                        println!("{}", gateway.mac_addr);
                        println!("{}", gateway.ip_addr);
                        return;
                    }
                    Err(_) => {
                        println!("Parse Error");
                    }
                },
                Err(e) => {
                    println!("{}", e);
                }
            }
            send_udp_packet();
        }
    }
}