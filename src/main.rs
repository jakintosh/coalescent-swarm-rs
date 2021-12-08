use identity::PeerInfo;
use kademlia::DHT;

mod hash_id;
mod identity;
mod kademlia;
mod networking;

fn main() {
    let peer_info = PeerInfo {
        address: networking::Interface::get_public_socket(),
        node_id: identity::Identity::generate_identity(),
    };
    let dht = DHT::new(20, peer_info);
}
