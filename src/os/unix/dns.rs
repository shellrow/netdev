use std::fs::read_to_string;
use std::net::{IpAddr, Ipv6Addr};

pub fn get_system_dns_conf() -> Vec<IpAddr> {
    read_to_string("/etc/resolv.conf")
        .map(|text| parse_nameservers(&text))
        .unwrap_or_default()
}

fn parse_nameservers(text: &str) -> Vec<IpAddr> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            if fields.next()? != "nameserver" {
                return None;
            }
            let server = fields.next()?;
            if let Some((ip, zone)) = server.split_once('%') {
                // The public DNS field stores IP addresses without IPv6 zone identifiers.
                if zone.is_empty() {
                    return None;
                }
                return ip.parse::<Ipv6Addr>().ok().map(IpAddr::V6);
            }
            server.parse().ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_numeric_resolvers_without_hostname_lookup() {
        let text = "# nameserver 192.0.2.1\nsearch example.org\nnameserver localhost\n\
                    nameserver 192.0.2.53 # local\nnameserver 2001:db8::53\nnameserver fe80::1%eth0";
        assert_eq!(
            parse_nameservers(text),
            ["192.0.2.53", "2001:db8::53", "fe80::1"].map(|s| s.parse::<IpAddr>().unwrap())
        );
    }
}
