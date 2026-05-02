use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn dhcp_enabled(iface_name: &str, ifindex: u32) -> Option<bool> {
    systemd_networkd_dhcp_enabled(ifindex)
        .or_else(|| network_manager_dhcp_enabled(iface_name))
        .or_else(|| dhclient_lease_detected(iface_name).then_some(true))
}

fn systemd_networkd_dhcp_enabled(ifindex: u32) -> Option<bool> {
    if ifindex == 0 {
        return None;
    }
    let path = PathBuf::from(format!("/run/systemd/netif/links/{ifindex}"));
    let content = fs::read_to_string(path).ok()?;
    parse_systemd_networkd_link(&content)
}

fn network_manager_dhcp_enabled(iface_name: &str) -> Option<bool> {
    for dir in [
        "/run/NetworkManager/system-connections",
        "/etc/NetworkManager/system-connections",
    ] {
        let Some(value) = network_manager_dir_dhcp_enabled(Path::new(dir), iface_name) else {
            continue;
        };
        return Some(value);
    }
    None
}

fn network_manager_dir_dhcp_enabled(dir: &Path, iface_name: &str) -> Option<bool> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let file_type = entry.file_type().ok()?;
        if !file_type.is_file() {
            continue;
        }
        let content = fs::read_to_string(entry.path()).ok()?;
        if let Some(value) = parse_network_manager_connection(&content, iface_name) {
            return Some(value);
        }
    }
    None
}

fn dhclient_lease_detected(iface_name: &str) -> bool {
    for dir in [
        "/run/NetworkManager",
        "/var/lib/NetworkManager",
        "/var/lib/dhcp",
        "/var/lib/dhclient",
    ] {
        if dhclient_lease_in_dir(Path::new(dir), iface_name) {
            return true;
        }
    }
    false
}

fn dhclient_lease_in_dir(dir: &Path, iface_name: &str) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.contains(iface_name) || !file_name.contains("lease") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(entry.path()) {
            if content.contains("lease") || content.contains("dhcp") {
                return true;
            }
        }
    }
    false
}

fn parse_systemd_networkd_link(content: &str) -> Option<bool> {
    for line in content.lines() {
        let Some((key, value)) = split_key_value(line) else {
            continue;
        };
        if key != "DHCP" {
            continue;
        }
        return parse_systemd_dhcp_value(value);
    }
    None
}

fn parse_systemd_dhcp_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "ipv4" => Some(true),
        "no" | "false" | "none" | "ipv6" => Some(false),
        _ => None,
    }
}

fn parse_network_manager_connection(content: &str, iface_name: &str) -> Option<bool> {
    let mut section = "";
    let mut connection_matches = false;
    let mut ipv4_method = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = name.trim();
            continue;
        }
        let Some((key, value)) = split_key_value(line) else {
            continue;
        };
        match (section, key) {
            ("connection", "interface-name") => {
                connection_matches = value == iface_name;
            }
            ("ipv4", "method") => {
                ipv4_method = Some(value);
            }
            _ => {}
        }
    }

    if !connection_matches {
        return None;
    }
    parse_network_manager_ipv4_method(ipv4_method?)
}

fn parse_network_manager_ipv4_method(method: &str) -> Option<bool> {
    match method.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(true),
        "manual" | "disabled" | "link-local" | "shared" => Some(false),
        _ => None,
    }
}

fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), value.trim()))
}

#[cfg(test)]
mod tests {
    use super::{
        parse_network_manager_connection, parse_network_manager_ipv4_method,
        parse_systemd_networkd_link,
    };

    #[test]
    fn parses_systemd_networkd_dhcp_values() {
        assert_eq!(
            parse_systemd_networkd_link("ADMIN_STATE=configured\nDHCP=yes\n"),
            Some(true)
        );
        assert_eq!(parse_systemd_networkd_link("DHCP=ipv4\n"), Some(true));
        assert_eq!(parse_systemd_networkd_link("DHCP=no\n"), Some(false));
        assert_eq!(parse_systemd_networkd_link("DHCP=ipv6\n"), Some(false));
        assert_eq!(parse_systemd_networkd_link("STATE=routable\n"), None);
    }

    #[test]
    fn parses_network_manager_connection_for_matching_interface() {
        let content = "\
[connection]
id=Wired
interface-name=eth0

[ipv4]
method=auto
";
        assert_eq!(
            parse_network_manager_connection(content, "eth0"),
            Some(true)
        );
        assert_eq!(parse_network_manager_connection(content, "wlan0"), None);
    }

    #[test]
    fn parses_network_manager_static_methods() {
        assert_eq!(parse_network_manager_ipv4_method("manual"), Some(false));
        assert_eq!(parse_network_manager_ipv4_method("disabled"), Some(false));
        assert_eq!(parse_network_manager_ipv4_method("unknown"), None);
    }
}
