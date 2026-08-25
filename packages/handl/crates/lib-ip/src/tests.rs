use crate::{Cidr, Cidr4, Cidr6, IpAddr, IpError, Ipv4, Ipv6, SocketAddr};

#[test]
fn error_display() {
    assert_eq!(IpError::InvalidAddress.to_string(), "invalid IP address");
    assert_eq!(IpError::InvalidCidr.to_string(), "invalid CIDR notation");
    assert_eq!(IpError::InvalidPort.to_string(), "invalid port number");
}

#[test]
fn ipv4_parse_and_display() {
    let ip = Ipv4::parse("192.168.1.1").unwrap();
    assert_eq!(ip.octets(), [192, 168, 1, 1]);
    assert_eq!(ip.to_u32(), 0xc0a80101);
    assert_eq!(ip.to_string(), "192.168.1.1");
    assert_eq!(Ipv4::from_u32(0xc0a80101), ip);
}

#[test]
fn ipv4_constructors() {
    let ip = Ipv4::new(10, 0, 0, 1);
    assert_eq!(ip, Ipv4::parse("10.0.0.1").unwrap());
    assert_eq!(ip.octets(), [10, 0, 0, 1]);
}

#[test]
fn ipv4_properties() {
    assert!(Ipv4::parse("127.0.0.1").unwrap().is_loopback());
    assert!(Ipv4::parse("10.10.0.1").unwrap().is_private());
    assert!(Ipv4::parse("172.20.0.1").unwrap().is_private());
    assert!(Ipv4::parse("192.168.1.1").unwrap().is_private());
    assert!(Ipv4::parse("169.254.1.1").unwrap().is_link_local());
    assert!(Ipv4::parse("224.0.0.1").unwrap().is_multicast());
    assert!(Ipv4::parse("0.0.0.0").unwrap().is_unspecified());
    assert!(Ipv4::parse("255.255.255.255").unwrap().is_broadcast());

    assert!(!Ipv4::parse("192.168.1.1").unwrap().is_loopback());
    assert!(!Ipv4::parse("192.168.1.1").unwrap().is_unspecified());
}

#[test]
fn ipv4_invalid() {
    assert!(Ipv4::parse("192.168.1").is_err());
    assert!(Ipv4::parse("192.168.1.1.1").is_err());
    assert!(Ipv4::parse("192.168.1.256").is_err());
    assert!(Ipv4::parse("192.168.01.1").is_err());
    assert!(Ipv4::parse("").is_err());
    assert!(Ipv4::parse("...").is_err());
}

#[test]
fn ipv6_parse_and_display() {
    let ip = Ipv6::parse("2001:db8::1").unwrap();
    assert_eq!(ip.to_string(), "2001:db8::1");

    let full = Ipv6::parse("1:2:3:4:5:6:7:8").unwrap();
    assert_eq!(full.to_string(), "1:2:3:4:5:6:7:8");

    let unspecified = Ipv6::parse("::").unwrap();
    assert_eq!(unspecified.to_string(), "::");

    let loopback = Ipv6::parse("::1").unwrap();
    assert_eq!(loopback.to_string(), "::1");
    assert_eq!(loopback.to_u128(), 1);

    let trailing = Ipv6::parse("1::").unwrap();
    assert_eq!(trailing.to_string(), "1::");

    let leading = Ipv6::parse("::1:2:3:4:5:6:7").unwrap();
    assert_eq!(leading.to_string(), "0:1:2:3:4:5:6:7");

    let middle = Ipv6::parse("1:2::3:4:5:6").unwrap();
    assert_eq!(middle.to_string(), "1:2::3:4:5:6");
}

#[test]
fn ipv6_constructors() {
    let ip = Ipv6::new(0, 0, 0, 0, 0, 0, 0, 1);
    assert_eq!(ip, Ipv6::parse("::1").unwrap());
    assert_eq!(ip.segments(), [0, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(Ipv6::from_u128(1), ip);
    assert_eq!(Ipv6::from_segments([0, 0, 0, 0, 0, 0, 0, 1]), ip);
}

#[test]
fn ipv6_properties() {
    assert!(Ipv6::parse("::1").unwrap().is_loopback());
    assert!(Ipv6::parse("::").unwrap().is_unspecified());
    assert!(Ipv6::parse("ff02::1").unwrap().is_multicast());

    assert!(!Ipv6::parse("2001:db8::1").unwrap().is_loopback());
    assert!(!Ipv6::parse("2001:db8::1").unwrap().is_unspecified());
    assert!(!Ipv6::parse("2001:db8::1").unwrap().is_multicast());
}

#[test]
fn ipv6_invalid() {
    assert!(Ipv6::parse("").is_err());
    assert!(Ipv6::parse("1:2:3:4:5:6:7").is_err());
    assert!(Ipv6::parse("1:2:3:4:5:6:7:8:9").is_err());
    assert!(Ipv6::parse("1::2::3").is_err());
    assert!(Ipv6::parse("1:2:3:4:5:6:7:8::").is_err());
    assert!(Ipv6::parse("gggg::1").is_err());
}

#[test]
fn ip_addr_parse_and_properties() {
    let v4 = IpAddr::parse("127.0.0.1").unwrap();
    assert!(v4.is_loopback());
    assert!(matches!(v4, IpAddr::V4(_)));
    assert_eq!(v4.to_string(), "127.0.0.1");

    let v6 = IpAddr::parse("::1").unwrap();
    assert!(v6.is_loopback());
    assert!(matches!(v6, IpAddr::V6(_)));
    assert_eq!(v6.to_string(), "::1");

    assert!(IpAddr::parse("not-an-address").is_err());
}

#[test]
fn cidr4_parse_and_display() {
    let cidr = Cidr4::parse("192.168.0.0/24").unwrap();
    assert_eq!(cidr.to_string(), "192.168.0.0/24");
    assert_eq!(cidr.network().to_string(), "192.168.0.0");
    assert_eq!(cidr.broadcast().to_string(), "192.168.0.255");
    assert_eq!(cidr.address_count(), 256);
    assert_eq!(cidr.host_count(), 254);
    assert_eq!(cidr.first_host().to_string(), "192.168.0.1");
    assert_eq!(cidr.last_host().to_string(), "192.168.0.254");
}

#[test]
fn cidr4_contains() {
    let cidr = Cidr4::parse("10.0.0.0/8").unwrap();
    let inside = Ipv4::parse("10.1.2.3").unwrap();
    let outside = Ipv4::parse("11.1.2.3").unwrap();
    assert!(cidr.contains(inside));
    assert!(!cidr.contains(outside));
}

#[test]
fn cidr4_edge_prefixes() {
    let all = Cidr4::parse("0.0.0.0/0").unwrap();
    assert!(all.contains(Ipv4::parse("255.255.255.255").unwrap()));
    assert_eq!(all.address_count(), 1u64 << 32);

    let host = Cidr4::parse("192.168.1.1/32").unwrap();
    assert_eq!(host.network().to_string(), "192.168.1.1");
    assert_eq!(host.broadcast().to_string(), "192.168.1.1");
    assert_eq!(host.host_count(), 0);
}

#[test]
fn cidr4_invalid() {
    assert!(Cidr4::parse("192.168.0.0").is_err());
    assert!(Cidr4::parse("192.168.0.0/33").is_err());
    assert!(Cidr4::parse("192.168.0.0/abc").is_err());
    assert!(Cidr4::parse("192.168.0.0/-1").is_err());
}

#[test]
fn cidr6_parse_and_display() {
    let cidr = Cidr6::parse("2001:db8::/32").unwrap();
    assert_eq!(cidr.to_string(), "2001:db8::/32");
    assert_eq!(cidr.network().to_string(), "2001:db8::");
    assert!(cidr.contains(Ipv6::parse("2001:db8:1::1").unwrap()));
    assert!(!cidr.contains(Ipv6::parse("2001:db9::1").unwrap()));
}

#[test]
fn cidr6_invalid() {
    assert!(Cidr6::parse("2001:db8::").is_err());
    assert!(Cidr6::parse("2001:db8::/129").is_err());
}

#[test]
fn cidr_enum() {
    let v4 = Cidr::parse("10.0.0.0/8").unwrap();
    assert_eq!(v4.to_string(), "10.0.0.0/8");
    let inside = IpAddr::parse("10.1.2.3").unwrap();
    assert!(v4.contains(&inside));

    let v6 = Cidr::parse("2001:db8::/32").unwrap();
    assert_eq!(v6.to_string(), "2001:db8::/32");
    let inside6 = IpAddr::parse("2001:db8:1::1").unwrap();
    assert!(v6.contains(&inside6));
    assert!(!v6.contains(&inside));

    assert_eq!(v4.prefix(), 8);
    assert_eq!(v6.prefix(), 32);
}

#[test]
fn cidr_enum_mismatch() {
    let v4 = Cidr::parse("10.0.0.0/8").unwrap();
    let v6 = IpAddr::parse("::1").unwrap();
    assert!(!v4.contains(&v6));
}

#[test]
fn socket_parse_and_display() {
    let v4 = SocketAddr::parse("127.0.0.1:8080").unwrap();
    assert!(v4.is_loopback());
    assert_eq!(v4.port(), 8080);
    assert_eq!(v4.ip().to_string(), "127.0.0.1");
    assert_eq!(v4.to_string(), "127.0.0.1:8080");

    let v6 = SocketAddr::parse("[::1]:443").unwrap();
    assert!(v6.is_loopback());
    assert_eq!(v6.port(), 443);
    assert_eq!(v6.ip().to_string(), "::1");
    assert_eq!(v6.to_string(), "[::1]:443");
}

#[test]
fn socket_invalid() {
    assert!(SocketAddr::parse("127.0.0.1").is_err());
    assert!(SocketAddr::parse("127.0.0.1:").is_err());
    assert!(SocketAddr::parse("127.0.0.1:abc").is_err());
    assert!(SocketAddr::parse("127.0.0.1:65536").is_err());
    assert!(SocketAddr::parse("[::1]").is_err());
    assert!(SocketAddr::parse("[::1]:").is_err());
    assert!(SocketAddr::parse("[::1]443").is_err());
    assert!(SocketAddr::parse("[not-v6]:80").is_err());
}
