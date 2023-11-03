// This example shows how to get the default gateway and its properties.

fn main() {
    match default_net::get_default_gateway() {
        Ok(gateway) => {
            println!("Default Gateway");
            println!("\tMAC: {}", gateway.mac_addr);
            println!("\tIP: {}", gateway.ip_addr);
        }
        Err(e) => {
            println!("{}", e);
        }
    }
}
