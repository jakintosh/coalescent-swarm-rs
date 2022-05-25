use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::sync::mpsc;

use crate::{data::HashId, identity::Agent};

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

pub type WireTx = mpsc::UnboundedSender<(SocketAddr, WireProtocol)>;
pub type WireRx = mpsc::UnboundedReceiver<(SocketAddr, WireProtocol)>;

#[derive(Serialize, Deserialize, Debug)]
pub struct WireProtocol {
    pub msg: ProtocolMessage,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ProtocolMessage {
    RequestConnection { agent_id: HashId },
    CloseConnection,
    RequestIdentity,
    ShareIdentity { agent: Agent },
}

pub struct MessageQueue {
    pub message_tx: WireTx,
    message_rx: WireRx,
}

impl MessageQueue {
    pub fn new() -> MessageQueue {
        let (tx, rx) = mpsc::unbounded_channel();
        MessageQueue {
            message_tx: tx,
            message_rx: rx,
        }
    }

    pub async fn listen(&mut self) {
        while let Some((addr, wire_msg)) = self.message_rx.recv().await {
            println!("  recv() from {} -> {:?}", addr, wire_msg);
        }
    }
}
