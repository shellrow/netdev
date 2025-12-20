[crates-badge]: https://img.shields.io/crates/v/netdev.svg
[crates-url]: https://crates.io/crates/netdev
[license-badge]: https://img.shields.io/crates/l/netdev.svg
[examples-url]: https://github.com/shellrow/netdev/tree/main/examples
[doc-url]: https://docs.rs/netdev/latest/netdev
[doc-interface-url]: https://docs.rs/netdev/latest/netdev/interface/interface/struct.Interface.html
[netdev-github-url]: https://github.com/shellrow/netdev
[default-net-github-url]: https://github.com/shellrow/default-net
[default-net-crates-io-url]: https://crates.io/crates/default-net

# netdev [![Crates.io][crates-badge]][crates-url] ![License][license-badge]
  
`netdev` provides a cross-platform API for network interface.

## Key Features
- Enumerate all available network interfaces
- Detect the default network interface
- Retrieve interface metadata:
    - Interface type
    - MAC address
    - IPv4 / IPv6 addresses and prefixes
    - MTU, flags, operational state, etc...
- Native traffic statistics (RX/TX bytes) for each interface
- Designed for **cross-platform**

See the [Interface][doc-interface-url] struct documentation for detail.

## Supported platform
- Linux
- macOS
- Windows
- Android
- iOS (and other Apple targets)
- BSDs

## Usage
Add `netdev` to your `Cargo.toml`
```toml
[dependencies]
netdev = "0.40"
```

For more details, see [examples][examples-url] or [doc][doc-url].  

## Project History
This crate was originally published as [default-net][default-net-crates-io-url] 
and later rebranded to `netdev` by the author myself for future expansion, clearer naming, and long-term maintenance.

## Tested on
- Linux
    - Ubuntu 
        - 24.04
        - 22.04
        - 21.10 
        - 20.04 
        - 18.04
    - Kali 
        - 2024.2
        - 2023.2
        - 2022.1
        - 2021.1
    - Arch 
        - 2024.05.01
- macOS (Apple Silicon)
    - 14.7.6
- macOS (Intel)
    - 13.4.1
    - 11.6
- Windows 
    - 11 24H2 26100.6584
    - 11 23H2 22631.4602
    - 11 Pro 22H2 22621.3155
    - 11 22H2 22621.3155
    - 10 21H2 19044.1586
- FreeBSD
    - 14
- Android (arm64)
    - 16.0
- Android (x86_64)
    - 16.0
- iOS
    - 18.6.2
    - 18.1.1
