mod connection;

pub use crate::networking::connection::{
    connect_ws, listen_ws, Connection, ConnectionHandle, WireMessage,
};

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
