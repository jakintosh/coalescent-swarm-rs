#![allow(dead_code)]

mod hash_id;
mod identity;
mod kademlia;
mod networking;

use std::net::SocketAddr;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() {
    tokio::select! {
        _ = start_local_ipc_server() => {
            println!("local IPC server failed, app exiting")
        },
        _= start_peer_server() => {
            println!("peer server failed, app exiting")
        }
    }
}

fn start_local_ipc_server() -> JoinHandle<()> {
    tokio::spawn(async move {
        let ipc_server_ip = [127, 0, 0, 1];
        let ipc_server_port = 5000;
        let ipc_server_address = SocketAddr::from((ipc_server_ip, ipc_server_port));

        // await exit of server, handle recoverable errors, restart if possible
        while let Err(error) = networking::listen_ws(ipc_server_address).await {
            match error {
                _ => break, // no errors are recoverable
            }
        }
    })
}

fn start_peer_server() -> JoinHandle<()> {
    tokio::spawn(async move {
        let peer_server_ip = networking::Interface::get_local_ip_address()
            .expect("fatal error: couldn't resolve local ip address; app cannot run.");
        let peer_server_port = 18300;
        let peer_server_address = SocketAddr::from((peer_server_ip, peer_server_port));

        // await exit of server, handle recoverable errors, restart if possible
        while let Err(error) = networking::listen_ws(peer_server_address).await {
            match error {
                _ => break, // no errors are recoverable
            }
        }
    })
}
