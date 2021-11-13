[crates-badge]: https://img.shields.io/crates/v/default-net.svg
[crates-url]: https://crates.io/crates/default-net
[license-badge]: https://img.shields.io/crates/l/default-net.svg
[examples-url]: https://github.com/shellrow/default-net/tree/main/examples
# default-net [![Crates.io][crates-badge]][crates-url] ![License][license-badge]
Get default network information  
`default-net` provides a cross-platform API for network interface and gateway.

## Supported platform
- Linux
- macOS
- Windows

## Usage
Add `default-net` to your dependencies  
```toml:Cargo.toml
[dependencies]
default-net = "0.5.0"
```

## Example 
```rust
use default_net;

fn main(){
    match default_net::get_default_interface() {
        Ok(default_interface) => {
            println!("Default Interface");
            println!("\tIndex: {}", default_interface.index);
            println!("\tName: {}", default_interface.name);
            if let Some(mac_addr) = default_interface.mac_addr {
                println!("\tMAC: {}", mac_addr);
            }else{
                println!("\tMAC: (Failed to get mac address)");
            }
            println!("\tIPv4: {:?}", default_interface.ipv4);
            println!("\tIPv6: {:?}", default_interface.ipv6);
            if let Some(gateway) = default_interface.gateway {
                println!("Default Gateway");
                println!("\tMAC: {}", gateway.mac_addr);
                println!("\tIP: {}", gateway.ip_addr);
            }else {
                println!("Default Gateway: (Not found)");
            }
        },
        Err(e) => {
            println!("{}", e);
        },
    }
}
```

For more details, see [examples][examples-url] or doc.  

