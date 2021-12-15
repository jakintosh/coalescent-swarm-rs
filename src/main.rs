#![allow(dead_code)]

use crate::identity::PeerInfo;
use crate::kademlia::DHT;

mod hash_id;
mod identity;
mod kademlia;
mod networking;

#[tokio::main]
async fn main() {
    // start running the IPC websocket server
    let ipc_server_listening_port = 5000;
    let ipc_server = networking::WebSocketServer::new_local(ipc_server_listening_port);
    let ipc_server_handle = ipc_server.start();

    // get this node's identifying information
    let identity = identity::Identity::generate_identity();
    let public_address = networking::Interface::get_public_socket();

    // get local peer info
    let peer_info = PeerInfo {
        address: public_address,
        node_id: identity,
    };

    // initialize the DHT
    let _dht = DHT::new(20, peer_info);

    // wait on the ipc server
    ipc_server_handle.await;
}
