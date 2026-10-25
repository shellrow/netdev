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

Cross-platform library for enumerating network interfaces with metadata.    
`netdev` provides a unified API for discovering local network interfaces
and retrieving commonly used metadata across platforms.

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

See the [Interface][doc-interface-url] struct documentation for more details.

## Supported platforms
- Linux
- macOS
- Windows
- Android
- iOS
- BSDs

## Usage
Add `netdev` to your `Cargo.toml`:
```toml
[dependencies]
netdev = "0.47"
```

For more details, see [examples][examples-url] or [doc][doc-url].  

## Feature flags
- `gateway` (default)
  - Enables default interface and default gateway helpers.
- `apple-system-configuration-extra` (default)
  - Enables additional `objc2-system-configuration` dependency bindings on macOS.
  - Currently, this flag does not change metadata collection: macOS reads interface names and DHCP hints from SystemConfiguration plist files, with a basic SystemConfiguration API fallback, regardless of this flag. It has no effect on iOS.
- `android-extra` (default)
  - Enables deeper Android metadata enrichment using Android platform APIs through JNI bindings.
  - On Android, this can add metadata such as traffic stats, DNS servers, DHCP hints, and Wi-Fi link speed when the app provides the required Android context and permissions.
- `apple-wifi-extra`
  - Enables macOS Wi-Fi transmit-speed collection through CoreWLAN.
  - This feature is disabled by default because CoreWLAN metadata collection may perform synchronous IPC during interface enumeration.

To disable default feature flags while keeping gateway helpers (macOS metadata collection remains enabled):

```toml
[dependencies]
netdev = { version = "0.47", default-features = false, features = ["gateway"] }
```

## Apple behavior
`netdev` links `SystemConfiguration.framework` automatically on `macOS` through its build script.
If your app is ultimately linked by Xcode, you may still need to add `SystemConfiguration.framework` to the app target manually.
On `iOS`, `netdev` uses only public APIs: `getifaddrs`, routing sysctls, `Network.framework` and libresolv.

macOS Wi-Fi transmit-speed collection is disabled by default. To enable it, opt in to the
`apple-wifi-extra` feature:

```toml
[dependencies]
netdev = { version = "0.47", features = ["apple-wifi-extra"] }
```

This uses CoreWLAN, whose metadata lookup may perform synchronous IPC while interfaces are
being enumerated.

## Android behavior
If you want Android-specific values such as DNS servers, DHCP hints, or Wi-Fi link speed, your app may still need to initialize the Android context for Rust and declare Android permissions such as `ACCESS_NETWORK_STATE` and `ACCESS_WIFI_STATE`.

## Project History
This crate was originally published as [default-net][default-net-crates-io-url]
and was later rebranded to `netdev` by the author for future expansion, clearer naming, and long-term maintenance.

## Tested on
- Linux
    - Ubuntu 
        - 24.04.5
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
    - 26.3.1
    - 14.7.6
- macOS (Intel)
    - 13.4.1
- Windows 
    - 11 25H2 26200.9711
    - 11 24H2 26100.6584
- FreeBSD
    - 14.4
- OpenBSD
    - 7.9
- NetBSD
    - 10.1
- Android (arm64)
    - 17
    - 16
- Android (x86_64)
    - 16
- iOS
    - 26.6.2
    - 18.6.2
