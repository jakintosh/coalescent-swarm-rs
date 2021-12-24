mod connection;

// pub(crate) use crate::networking::connection::connect_ws;
pub(crate) use crate::networking::connection::listen_ws;

use std::net::{IpAddr, SocketAddr};

pub struct Interface;
impl Interface {
    pub fn get_public_socket() -> Option<SocketAddr> {
        None
    }
    pub fn get_local_ip_address() -> Option<IpAddr> {
        match local_ip_address::local_ip() {
            Ok(ip_address) => Some(ip_address),
            Err(_) => None,
        }
    }
}
