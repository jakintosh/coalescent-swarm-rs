#![allow(dead_code)]

mod hash_id;
mod identity;
mod kademlia;
mod networking;

use tokio::task::JoinHandle;

#[tokio::main]
async fn main() {
    let _ = tokio::join!(start_local_ipc_server(), start_peer_server());
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
}

fn start_local_ipc_server() -> JoinHandle<()> {
    tokio::spawn(async move {
        let ipc_server_port = 5000;

        // await exit of server, handle recoverable errors, restart if possible
        while let Err(error) = networking::websocket::listen_localhost(ipc_server_port).await {
            match error {
                _ => break, // no errors are recoverable
            }
        }
    })
}

fn start_peer_server() -> JoinHandle<()> {
    tokio::spawn(async move {
        let peer_server_port = 18300;
        let peer_server_ip = networking::Interface::get_local_ip_address()
            .expect("fatal error: couldn't resolve local ip address; app cannot run.");
        let peer_server_address = std::net::SocketAddr::from((peer_server_ip, peer_server_port));

        // await exit of server, handle recoverable errors, restart if possible
        while let Err(error) = networking::websocket::listen(peer_server_address).await {
            match error {
                _ => break, // no errors are recoverable
            }
        }
    })
}
