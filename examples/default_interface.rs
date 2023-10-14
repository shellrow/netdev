use default_net;

fn main() {
    match default_net::get_default_interface() {
        Ok(interface) => {
            println!("Default Interface");
            println!("\tIndex: {}", interface.index);
            println!("\tName: {}", interface.name);
            println!("\tFriendly Name: {:?}", interface.friendly_name);
            println!("\tDescription: {:?}", interface.description);
            println!("\tType: {}", interface.if_type.name());
            println!("\tFlags: {:?}", interface.flags);
            println!("\t\tis UP {}", interface.is_up());
            println!("\t\tis LOOPBACK {}", interface.is_loopback());
            println!("\t\tis MULTICAST {}", interface.is_multicast());
            println!("\t\tis BROADCAST {}", interface.is_broadcast());
            println!("\t\tis POINT TO POINT {}", interface.is_point_to_point());
            println!("\t\tis TUN {}", interface.is_tun());
            if let Some(mac_addr) = interface.mac_addr {
                println!("\tMAC: {}", mac_addr);
            } else {
                println!("\tMAC: (Failed to get mac address)");
            }
            println!("\tIPv4: {:?}", interface.ipv4);
            println!("\tIPv6: {:?}", interface.ipv6);
            println!("\tTransmit Speed: {:?}", interface.transmit_speed);
            println!("\tReceive Speed: {:?}", interface.receive_speed);
            if let Some(gateway) = interface.gateway {
                println!("Default Gateway");
                println!("\tMAC: {}", gateway.mac_addr);
                println!("\tIP: {}", gateway.ip_addr);
            } else {
                println!("Default Gateway: (Not found)");
            }
        }
        Err(e) => {
            println!("{}", e);
        }
    }
}
