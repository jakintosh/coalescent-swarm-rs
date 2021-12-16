#![allow(dead_code)]

use crate::identity::PeerInfo;
use crate::kademlia::DHT;

mod hash_id;
mod identity;
mod kademlia;
mod networking;

#[tokio::main]
async fn main() {
    // start the local IPC websocket server
    let ipc_server_listening_port = 5000;
    let ipc_server_handle = networking::WebSocketServer::listen_local(ipc_server_listening_port);

    // start a public websocket server for peers
    let peer_server_local_ip = networking::Interface::get_local_ip_address()
        .expect("fatal error: couldn't resolve local ip address; app cannot run.");
    let peer_server_port = 18300;
    let peer_server_address = std::net::SocketAddr::from((peer_server_local_ip, peer_server_port));
    let peer_server_handle = networking::WebSocketServer::listen(peer_server_address);

    // get this node's identifying information
    // let identity = identity::Identity::generate_identity();
    // let public_address = networking::Interface::get_public_socket();

    // get local peer info
    // let peer_info = PeerInfo {
    // address: public_address,
    // node_id: identity,
    // };

    // initialize the DHT
    // let _dht = DHT::new(20, peer_info);

    // wait on the servers
    futures::pin_mut!(ipc_server_handle, peer_server_handle);
    futures::future::select(ipc_server_handle, peer_server_handle).await;
}
