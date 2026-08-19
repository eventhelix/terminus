//! Real IPv4/UDP bytes via etherparse (design §2: standard protocols
//! are real encodings carried inside the PDL framing; Wireshark chains
//! its built-in dissectors after the PDL layers).

use etherparse::{NetHeaders, PacketBuilder, PacketHeaders, TransportHeader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpView {
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
    pub src_port: u16,
    pub dst_port: u16,
    pub payload: Vec<u8>,
}

/// Build a full IPv4+UDP packet (no Ethernet — the LinkFrame is our
/// link layer). Panics on failure: failure to encode is a model bug.
pub fn build_udp_ipv4(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    ttl: u8,
    payload: &[u8],
) -> Vec<u8> {
    let builder = PacketBuilder::ipv4(src_ip, dst_ip, ttl).udp(src_port, dst_port);
    let mut bytes = Vec::with_capacity(builder.size(payload.len()));
    builder.write(&mut bytes, payload).expect("UDP/IPv4 encode is infallible for Vec");
    bytes
}

/// Parse bytes that should start at an IPv4 header. `None` = not clean
/// IPv4+UDP — callers count this as a network reality, never an error.
pub fn parse_udp_ipv4(bytes: &[u8]) -> Option<UdpView> {
    let h = PacketHeaders::from_ip_slice(bytes).ok()?;
    let (src_ip, dst_ip) = match h.net? {
        NetHeaders::Ipv4(ip, _) => (ip.source, ip.destination),
        _ => return None,
    };
    let udp = match h.transport? {
        TransportHeader::Udp(u) => u,
        _ => return None,
    };
    Some(UdpView {
        src_ip,
        dst_ip,
        src_port: udp.source_port,
        dst_port: udp.destination_port,
        payload: h.payload.slice().to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let payload = [1u8, 2, 3, 4];
        let bytes = build_udp_ipv4([10, 0, 0, 1], [10, 0, 0, 6], 4001, 7, 64, &payload);
        assert_eq!(bytes[0] >> 4, 4, "IPv4 version nibble");
        let v = parse_udp_ipv4(&bytes).unwrap();
        assert_eq!(v.src_ip, [10, 0, 0, 1]);
        assert_eq!(v.dst_ip, [10, 0, 0, 6]);
        assert_eq!((v.src_port, v.dst_port), (4001, 7));
        assert_eq!(v.payload, payload);
    }

    #[test]
    fn garbage_parses_to_none() {
        assert!(parse_udp_ipv4(&[0xDE, 0xAD, 0xBE, 0xEF]).is_none());
        assert!(parse_udp_ipv4(&[]).is_none());
    }
}
