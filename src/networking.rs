use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub struct Interface;

impl Interface {
    pub fn get_public_socket() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000)
    }
}
