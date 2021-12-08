use crate::hash_id::*;
use std::net::SocketAddr;

#[derive(PartialEq, Copy, Clone)]
pub struct PeerInfo {
    pub address: SocketAddr,
    pub node_id: HashId,
}

pub struct Identity;
impl Identity {
    pub fn generate_identity() -> HashId {
        HashId([0u32; 8])
    }
}
