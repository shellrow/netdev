use default_net;

fn main(){
    let interfaces = default_net::get_interfaces();
    for interface in interfaces {
        println!("Interface");
        println!("\tIndex: {}", interface.index);
        println!("\tName: {}", interface.name);
        println!("\tDescription: {:?}", interface.description);
        println!("\tType: {}", interface.if_type.name());
        if let Some(mac_addr) = interface.mac_addr {
            println!("\tMAC: {}", mac_addr);
        }else{
            println!("\tMAC: (Failed to get mac address)");
        }
        println!("\tIPv4: {:?}", interface.ipv4);
        println!("\tIPv6: {:?}", interface.ipv6);
        println!("\tFlags: {:?}", interface.flags);
        if let Some(gateway) = interface.gateway {
            println!("Gateway");
            println!("\tMAC: {}", gateway.mac_addr);
            println!("\tIP: {}", gateway.ip_addr);
        }else {
            println!("Gateway: (Not found)");
        }
        println!();
    }
}
