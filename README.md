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
default-net = "0.6.0"
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
            println!("\tDescription: {:?}", default_interface.description);
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

## Tested on
- Linux
    - Ubuntu 20.04, 18.04
    - Kali 2021.1 (VM)
- macOS 11.6
- Windows 10 20H2

For more details, see [examples][examples-url] or doc.  


## For Windows users using v0.5.0 or lower
To build [libpnet](https://github.com/libpnet/libpnet) on Windows, follow the instructions below.
> ### Windows
> * You must use a version of Rust which uses the MSVC toolchain
> * You must have [WinPcap](https://www.winpcap.org/) or [npcap](https://nmap.org/npcap/) installed
>   (tested with version WinPcap 4.1.3) (If using npcap, make sure to install with the "Install Npcap in WinPcap API-compatible Mode")
> * You must place `Packet.lib` from the [WinPcap Developers pack](https://www.winpcap.org/devel.htm)
>   in a directory named `lib`, in the root of this repository. Alternatively, you can use any of the
>   locations listed in the `%LIB%`/`$Env:LIB` environment variables. For the 64 bit toolchain it is
>   in `WpdPack/Lib/x64/Packet.lib`, for the 32 bit toolchain, it is in `WpdPack/Lib/Packet.lib`.

[Source](https://github.com/libpnet/libpnet/blob/master/README.md#windows "libpnet#windows")
