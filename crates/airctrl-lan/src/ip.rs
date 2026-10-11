use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use crate::error::LanError;

/// Discovers the local physical LAN IPv4 address using a zero-packet UDP socket inquiry.
///
/// Under POSIX and Windows socket implementations, invoking `connect` on a UDP socket
/// does not transmit any packets onto the physical network wire (no SYN, no ARP, no datagram).
/// Instead, the operating system kernel performs a Forwarding Information Base (FIB)
/// routing table lookup to determine the egress network interface and binds the socket
/// to that interface's IP. Calling `local_addr()` then extracts the non-loopback IP address.
///
/// If probing external addresses fails (e.g. host is completely offline with no routes),
/// it falls back to 127.0.0.1.
pub fn get_lan_ip() -> Result<Ipv4Addr, LanError> {
    let probe_targets = [
        "8.8.8.8:80",
        "1.1.1.1:80",
        "192.168.1.1:80",
        "10.0.0.1:80",
        "172.16.0.1:80",
    ];

    for target in probe_targets {
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if socket.connect(target).is_ok() {
                if let Ok(SocketAddr::V4(addr)) = socket.local_addr() {
                    let ip = *addr.ip();
                    if !ip.is_loopback() && !ip.is_unspecified() {
                        return Ok(ip);
                    }
                }
            }
        }
    }

    Ok(Ipv4Addr::LOCALHOST)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lan_ip_discovery_returns_valid_ip() {
        let ip = get_lan_ip().expect("Failed to retrieve LAN IP");
        assert!(!ip.is_unspecified(), "Discovered IP cannot be 0.0.0.0");
    }
}
