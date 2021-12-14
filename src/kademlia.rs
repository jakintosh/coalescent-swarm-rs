use crate::hash_id::HashId;
use crate::identity::PeerInfo;
use std::collections::HashMap;

pub enum Message {
    Ping(PeerInfo),
    Store(PeerInfo),
    FindNode(PeerInfo, HashId),
    FindValue(PeerInfo, HashId),
}

pub struct DHT {
    my_peer_info: PeerInfo,
    bucket_limit: u8,
    k_buckets: HashMap<u8, Vec<PeerInfo>>,
}

impl DHT {
    pub fn new(bucket_limit: u8, peer_info: PeerInfo) -> DHT {
        DHT {
            my_peer_info: peer_info,
            bucket_limit,
            k_buckets: HashMap::new(),
        }
    }
    fn get_length_of_matching_prefix(&self, id_1: &HashId) -> u8 {
        self.my_peer_info.node_id.leading_zeros(&id_1) as u8
    }
    fn get_or_create_k_bucket_for_id(&mut self, hash_id: &HashId) -> &mut Vec<PeerInfo> {
        let matched_prefix_length = self.get_length_of_matching_prefix(hash_id);
        self.k_buckets
            .entry(matched_prefix_length)
            .or_insert(Vec::new())
    }
    fn witness_peer(&mut self, peer_info: &PeerInfo) {
        let bucket_limit = self.bucket_limit as usize;
        let k_bucket = self.get_or_create_k_bucket_for_id(&peer_info.node_id);
        match k_bucket.iter().position(|peer| peer == peer_info) {
            Some(index) => {
                // move known node to most recently seen
                let element = k_bucket.remove(index);
                k_bucket.push(element);
            }
            None => {
                if k_bucket.len() < bucket_limit {
                    // add unknown node to most recently seen
                    k_bucket.push(peer_info.clone());
                } else {
                    let eviction_candidate = k_bucket.first().unwrap();
                    // ping eviction candidate
                    // if responds, keep and bump to end
                    // if no response, evict and add new peer to end
                }
            }
        }
    }
}
