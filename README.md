[crates-badge]: https://img.shields.io/crates/v/default-net.svg
[crates-url]: https://crates.io/crates/default-net
[license-badge]: https://img.shields.io/crates/l/default-net.svg
[examples-url]: https://github.com/shellrow/default-net/tree/main/examples
# default-net [![Crates.io][crates-badge]][crates-url] ![License][license-badge]
  
`default-net` provides a cross-platform API for network interface and gateway.

- Get default Network Interface and Gateway information
- Get list of available Network Interfaces

## Supported platform
- Linux
- macOS
- Windows

## Usage
Add `default-net` to your dependencies  
```toml:Cargo.toml
[dependencies]
default-net = "0.9.0"
```

## Example 
The following example retrieves and displays information about the default network interface.
```rust
use default_net;

fn main(){
    match default_net::get_default_interface() {
        Ok(default_interface) => {
            println!("Default Interface");
            println!("\tIndex: {}", default_interface.index);
            println!("\tName: {}", default_interface.name);
            println!("\tDescription: {:?}", default_interface.description);
            println!("\tType: {}", default_interface.if_type.name());
            if let Some(mac_addr) = default_interface.mac_addr {
                println!("\tMAC: {}", mac_addr);
            }else{
                println!("\tMAC: (Failed to get mac address)");
            }
            println!("\tIPv4: {:?}", default_interface.ipv4);
            println!("\tIPv6: {:?}", default_interface.ipv6);
            println!("\tFlags: {:?}", default_interface.flags);
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

## Tested on
- Linux
    - Ubuntu 
        - 21.10 
        - 20.04 
        - 18.04
    - Kali 
        - 2022.1 (VM)
        - 2021.1 (VM)
- macOS 11.6
- Windows 
  - Windows 10 21H2 19044.1586
  - Windows 11 21H2 22000.493 (VM)

For more details, see [examples][examples-url] or doc.  
