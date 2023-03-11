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
default-net = "0.13"
```

For more details, see [examples][examples-url] or doc.  

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
  - Windows 11 Pro 21H2 22000.1335
  