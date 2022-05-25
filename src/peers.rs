use std::net::SocketAddr;

use crate::data::HashId;

/// known peers
///
/// we have some way to keep track of all known peers. is this kademlia?

/// active peers
///
/// we have some way to keep track of all the active peer connections we
/// are maintaining
struct Connection {
    address: SocketAddr,
    agent_id: HashId,
}
