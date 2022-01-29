// use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

// ===== enum messaging::WireMessage =========================================
///
/// This is the raw coalescent-swarm message that we send across the wire.
/// It will ultimately be wrapped by the protocol that transmits the message,
/// but at the coalescent-swarm abstraction layer, this is as low as it gets.
///
#[derive(Serialize, Deserialize)]
pub enum WireMessage {
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    RequestConnection,
    CloseConnection,
    ApiCall { function: String, data: Vec<u8> },
    Empty,
}

// /// recv msg
// fn receive_message(from: SocketAddr, message: WireMessage) {}
