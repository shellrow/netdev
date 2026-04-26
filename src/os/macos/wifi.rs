use objc2_core_wlan::CWWiFiClient;
use objc2_foundation::NSString;

/// Returns the macOS Wi-Fi transmit rate in bps for the given interface name.
pub(crate) fn get_wifi_transmit_rate(iface_name: &str) -> Option<u64> {
    let client = unsafe { CWWiFiClient::sharedWiFiClient() };
    let name = NSString::from_str(iface_name);

    let wifi_iface = unsafe { client.interfaceWithName(Some(&name)) };
    wifi_iface.map(|i| {
        let transmit_rate = unsafe { i.transmitRate() };
        return (transmit_rate * 1e6) as u64;
    })
}
