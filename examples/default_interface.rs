use default_net;

fn main() {
    match default_net::get_default_interface() {
        Ok(default_interface) => {
            println!("Default Interface");
            println!("\tIndex: {}", default_interface.index);
            println!("\tName: {}", default_interface.name);
            println!("\tFriendly Name: {:?}", default_interface.friendly_name);
            println!("\tDescription: {:?}", default_interface.description);
            println!("\tType: {}", default_interface.if_type.name());
            if let Some(mac_addr) = default_interface.mac_addr {
                println!("\tMAC: {}", mac_addr);
            } else {
                println!("\tMAC: (Failed to get mac address)");
            }
            println!("\tIPv4: {:?}", default_interface.ipv4);
            println!("\tIPv6: {:?}", default_interface.ipv6);
            println!("\tFlags: {:?}", default_interface.flags);
            println!("\tTransmit Speed: {:?}", default_interface.transmit_speed);
            println!("\tReceive Speed: {:?}", default_interface.receive_speed);
            if let Some(gateway) = default_interface.gateway {
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
