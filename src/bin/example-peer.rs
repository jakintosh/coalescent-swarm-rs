use coalescent_swarm::identity::Agent;
use coalescent_swarm::messaging::{self, ProtocolMessage, WireProtocol};
use coalescent_swarm::networking;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let peer_node_ip = networking::get_local_ip_address()
        .expect("fatal error: couldn't resolve local ip address; app cannot run.");
    let peer_node_port = 18351;
    let udp_peer_node_address = SocketAddr::from((peer_node_ip, peer_node_port));

    let mut message_queue = messaging::MessageQueue::new();
    let wire_tx = message_queue.message_tx.clone();

    let node = networking::Node::new(udp_peer_node_address, wire_tx).await;
    let mut listener_node = node.clone();
    let sender_node = node;

    let (agent, _persona) = Agent::create_with_new_persona();

    let dest = SocketAddr::from((peer_node_ip, 18350));
    let msg = WireProtocol {
        msg: ProtocolMessage::RequestConnection { agent_id: agent.id },
    };

    let net_handle = tokio::spawn(async move { listener_node.listen().await });
    let msg_handle = tokio::spawn(async move { message_queue.listen().await });
    let snd_handle = tokio::spawn(async move { sender_node.send(dest, msg).await });

    tokio::select! {
        _ = net_handle => {},
        _ = msg_handle => {},
        _ = snd_handle => {},
    }

    println!("exiting");
}
