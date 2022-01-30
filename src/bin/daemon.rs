use coalescent_swarm::{
    messaging::{MessageQueue, WireTx},
    networking,
};
use std::net::{self, SocketAddr};
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() {
    let message_queue = MessageQueue::new();
    let message_tx = message_queue.message_tx.clone();

    // TODO: redesign this to pass one-shot failure channels in, then
    //       await on a message from failures, then alert all subsystems
    //       about the failure to exit gracefully
    tokio::select! {
        _ = start_message_queue(message_queue) => {
            println!("message queue failed, app exiting")
        },
        _ = start_local_ipc_server() => {
            println!("local IPC server failed, app exiting")
        },
        _ = start_peer_server() => {
            println!("peer WS server failed, app exiting")
        },
        _ = start_peer_udp_server(message_tx) => {
            println!("peer udp server failed, app exiting")
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
        let peer_server_ip = networking::get_local_ip_address()
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

fn start_message_queue(mut message_queue: MessageQueue) -> JoinHandle<()> {
    tokio::spawn(async move {
        message_queue.listen().await;
    })
}

fn start_peer_udp_server(wire_msg_tx: WireTx) -> JoinHandle<()> {
    tokio::spawn(async move {
        let peer_node_ip = networking::get_local_ip_address()
            .expect("fatal error: couldn't resolve local ip address; app cannot run.");
        let peer_node_port = 18350;
        let udp_peer_node_address = SocketAddr::from((peer_node_ip, peer_node_port));

        // await exit of listener, handle recoverable errors, restart if possible
        let mut node = networking::Node::new(udp_peer_node_address, wire_msg_tx).await;
        while let Err(err) = node.listen().await {
            match err {
                _ => break, // no errors are recoverable
            }
        }
    })
}
