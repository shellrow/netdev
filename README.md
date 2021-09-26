[crates-badge]: https://img.shields.io/crates/v/default-net.svg
[crates-url]: https://crates.io/crates/default-net
[license-badge]: https://img.shields.io/crates/l/default-net.svg
[examples-url]: https://github.com/shellrow/default-net/tree/main/examples
# default-net [![Crates.io][crates-badge]][crates-url] ![License][license-badge]
Get default network information  
`default-net` provides a cross-platform API for network interface and gateway.

## Supported platform
- Linux
- macOS(OS X)
- Windows

## Usage
Add `default-net` to your dependencies  
```toml:Cargo.toml
[dependencies]
default-net = "0.4.0"
```

## Example 
```rust
use default_net;

fn main(){
    if let Some(default_interface) = default_net::get_default_interface(){
        println!("Index {}", default_interface.index);
        println!("Name {}", default_interface.name);
        println!("MAC {:?}", default_interface.mac);
        println!("IPv4 {:?}", default_interface.ipv4);
        println!("IPv6 {:?}", default_interface.ipv6);
        println!("Gateway IP {:?}", default_interface.gateway.ip);
        println!("Gateway MAC {:?}", default_interface.gateway.mac);
    }else{
        println!("Failed to get default interface info");
    }
}
```

For more details, see [examples][examples-url] or doc.  

