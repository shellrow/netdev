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
default-net = "0.21"
```

For more details, see [examples][examples-url] or doc.  

## Tested on
- Linux
    - Ubuntu 
        - 22.04
        - 21.10 
        - 20.04 
        - 18.04
    - Kali 
        - 2023.2
        - 2022.1
        - 2021.1
- macOS 
    - 13.4.1
    - 11.6
- Windows 
    - 11 Pro 22H2 22621.1848
    - 11 21H2 22000.493
    - 10 21H2 19044.1586