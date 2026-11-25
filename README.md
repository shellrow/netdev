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
- iOS and tvOS
- BSDs

## Usage
Add `netdev` to your `Cargo.toml`:
```toml
[dependencies]
netdev = "0.48"
```

## Cargo features

| Feature | Default | Effect |
| --- | --- | --- |
| `gateway` | Yes | Adds default-interface and gateway lookups, gateway addresses, and DNS servers. |
| `android-extra` | Yes | Uses Android APIs through JNI for additional traffic, DNS, DHCP, and Wi-Fi metadata. |
| `apple-system-configuration-extra` | Yes | Enables additional macOS SystemConfiguration bindings. Interface metadata collection also works with this feature disabled. |
| `apple-wifi-extra` | No | Reads macOS Wi-Fi transmit speed through CoreWLAN. This can perform synchronous IPC during enumeration. |
| `serde` | No | Enables serialization and deserialization of interface data. | 

For more details, see [examples][examples-url] or [doc][doc-url]. 

## Platform notes

- Linux uses routes and destination rules to choose a UDP lookup destination
  outside more-specific prefixes when possible. The kernel selects the route.
  When the table is unavailable, it uses a fixed destination, as do several other
  platforms. A split VPN can affect that fallback.
- On Android, the host must initialize `ndk-context` before the first interface
  or traffic-counter lookup with `android-extra`. Some Android runtimes do this
  during startup.
  A missing context aborts the process in builds using `panic = "abort"`.
  Android network and Wi-Fi metadata also depend on app permissions such as
  `ACCESS_NETWORK_STATE` and `ACCESS_WIFI_STATE`. For standalone binaries without
  an Android context, use `default-features = false` and enable `gateway` as needed.
- On macOS, Xcode targets that perform the final link may need to link
  `SystemConfiguration.framework`. On macOS, iOS and tvOS, the native byte counters
  wrap at 4 GiB.
- On illumos, traffic counters, gateway addresses, and DNS servers are unavailable.
  The default-interface helper can still identify the interface by its source address.

## Project History
This crate was originally published as [default-net][default-net-crates-io-url]
and was later rebranded to `netdev` by the author for future expansion, clearer naming, and long-term maintenance.
