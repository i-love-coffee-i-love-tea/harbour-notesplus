//! Shared network address classification utilities.

use std::net::{IpAddr, Ipv4Addr};

/// Returns true if the IP belongs to any non-routable, private, or reserved range.
/// This is the canonical IP classification used for SSRF protection, CORS, and access control.
pub fn is_blocked_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            // Loopback (127.0.0.0/8)
            if v4.is_loopback() || octets[0] == 127 {
                return true;
            }
            // Unspecified / Current network (0.0.0.0/8)
            if v4.is_unspecified() || octets[0] == 0 {
                return true;
            }
            // RFC 1918 10.0.0.0/8
            if octets[0] == 10 {
                return true;
            }
            // RFC 1918 172.16.0.0/12
            if octets[0] == 172 && (16..=31).contains(&octets[1]) {
                return true;
            }
            // RFC 1918 192.168.0.0/16
            if octets[0] == 192 && octets[1] == 168 {
                return true;
            }
            // Link-local / Cloud metadata (169.254.0.0/16)
            if v4.is_link_local() || (octets[0] == 169 && octets[1] == 254) {
                return true;
            }
            // Carrier-grade NAT (100.64.0.0/10)
            if octets[0] == 100 && (64..=127).contains(&octets[1]) {
                return true;
            }
            // IETF Protocol Assignments (192.0.0.0/24)
            if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
                return true;
            }
            // Documentation TEST-NET-1 (192.0.2.0/24)
            if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
                return true;
            }
            // Documentation TEST-NET-2 (198.51.100.0/24)
            if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
                return true;
            }
            // Documentation TEST-NET-3 (203.0.113.0/24)
            if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
                return true;
            }
            // Benchmarking (198.18.0.0/15)
            if octets[0] == 198 && (18..=19).contains(&octets[1]) {
                return true;
            }
            // Multicast (224.0.0.0/4) & Reserved (240.0.0.0/4) & Broadcast
            if v4.is_multicast() || v4.is_broadcast() || octets[0] >= 224 {
                return true;
            }
            false
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            // Loopback (::1)
            if v6.is_loopback() {
                return true;
            }
            // Unspecified (::)
            if v6.is_unspecified() {
                return true;
            }
            // IPv4-mapped IPv6 (::ffff:0:0/96)
            if let Some(v4) = v6.to_ipv4() {
                return is_blocked_ip(&IpAddr::V4(v4));
            }
            // Unique Local Address ULA (fc00::/7)
            if (segments[0] & 0xfe00) == 0xfc00 {
                return true;
            }
            // Link-local unicast (fe80::/10)
            if (segments[0] & 0xffc0) == 0xfe80 {
                return true;
            }
            // Multicast (ff00::/8)
            if v6.is_multicast() || (segments[0] & 0xff00) == 0xff00 {
                return true;
            }
            // Documentation (2001:db8::/32)
            if segments[0] == 0x2001 && segments[1] == 0x0db8 {
                return true;
            }
            // Discard prefix (100::/64)
            if segments[0] == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0 {
                return true;
            }
            false
        }
    }
}

/// Checks if an IP address belongs to a private, loopback, or local link network.
/// Subset of `is_blocked_ip` — used for CORS origin validation and public IP rejection.
pub fn is_private_or_local_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            if octets[0] == 127 { return true; }
            if octets[0] == 10 { return true; }
            if octets[0] == 172 && (16..=31).contains(&octets[1]) { return true; }
            if octets[0] == 192 && octets[1] == 168 { return true; }
            if octets[0] == 169 && octets[1] == 254 { return true; }
            if octets[0] == 100 && (64..=127).contains(&octets[1]) { return true; }
            if ipv4.is_unspecified() || ipv4.is_broadcast() { return true; }
            false
        }
        IpAddr::V6(ipv6) => {
            if ipv6.is_loopback() || ipv6.is_unspecified() { return true; }
            let octets = ipv6.octets();
            if octets[0..10] == [0; 10] && octets[10] == 0xff && octets[11] == 0xff {
                let v4 = std::net::Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]);
                return is_private_or_local_ip(&IpAddr::V4(v4));
            }
            if (octets[0] & 0xfe) == 0xfc { return true; }
            if octets[0] == 0xfe && (octets[1] & 0xc0) == 0x80 { return true; }
            false
        }
    }
}

/// Checks if an IP address is a public routable Internet address.
pub fn is_public_ip(ip: &IpAddr) -> bool {
    !is_private_or_local_ip(ip)
}

/// Parses alternative direct IP representations (standard, hex 0x, decimal, octal 0o, dotted).
pub fn parse_direct_ip(host: &str) -> Option<IpAddr> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Some(ip);
    }
    if let Some(hex) = host.strip_prefix("0x") {
        if let Ok(num) = u32::from_str_radix(hex, 16) {
            return Some(IpAddr::V4(Ipv4Addr::from(num)));
        }
    }
    if let Some(oct) = host.strip_prefix("0o") {
        if let Ok(num) = u32::from_str_radix(oct, 8) {
            return Some(IpAddr::V4(Ipv4Addr::from(num)));
        }
    }
    if let Ok(num) = host.parse::<u32>() {
        return Some(IpAddr::V4(Ipv4Addr::from(num)));
    }
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4 {
        let mut octets = [0u8; 4];
        let mut valid = true;
        for (i, part) in parts.iter().enumerate() {
            let val = if let Some(hex) = part.strip_prefix("0x") {
                u32::from_str_radix(hex, 16).ok()
            } else if let Some(oct) = part.strip_prefix("0o") {
                u32::from_str_radix(oct, 8).ok()
            } else {
                part.parse::<u32>().ok()
            };
            if let Some(v) = val {
                if v <= 255 {
                    octets[i] = v as u8;
                } else {
                    valid = false;
                    break;
                }
            } else {
                valid = false;
                break;
            }
        }
        if valid {
            return Some(IpAddr::V4(Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3])));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_private_ranges() {
        assert!(is_blocked_ip(&"10.0.0.1".parse().unwrap()));
        assert!(is_blocked_ip(&"192.168.1.1".parse().unwrap()));
        assert!(is_blocked_ip(&"172.16.0.1".parse().unwrap()));
        assert!(is_blocked_ip(&"127.0.0.1".parse().unwrap()));
        assert!(is_blocked_ip(&"169.254.169.254".parse().unwrap()));
    }

    #[test]
    fn allowed_public_ips() {
        assert!(!is_blocked_ip(&"93.184.216.34".parse().unwrap()));
        assert!(!is_blocked_ip(&"8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn blocked_ipv6() {
        assert!(is_blocked_ip(&"::1".parse().unwrap()));
        assert!(is_blocked_ip(&"fd00::1".parse().unwrap()));
        assert!(is_blocked_ip(&"fe80::1".parse().unwrap()));
    }

    #[test]
    fn public_ip_check() {
        assert!(is_public_ip(&"8.8.8.8".parse().unwrap()));
        assert!(!is_public_ip(&"192.168.1.1".parse().unwrap()));
    }
}
