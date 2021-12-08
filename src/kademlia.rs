mod kademlia {

    use std::{collections::HashMap, time::Instant};

    #[derive(PartialEq, Clone, Copy)]
    pub struct HashId(pub [u32; 8]);
    impl HashId {
        pub fn xor(&self, other: &HashId) -> [u32; 8] {
            let mut xor: [u32; 8] = [0; 8];
            for chunk in 0..8 {
                xor[chunk] = self.0[chunk] ^ other.0[chunk];
            }
            xor
        }
        fn leading_zeros(&self, other: &HashId) -> usize {
            let xor = self.xor(other);
            for chunk in 0..8 {
                let diff = xor[chunk];
                let leading_zeros = diff.leading_zeros();
                if leading_zeros < 32 {
                    return (chunk * 32) + leading_zeros as usize;
                }
            }
            256
        }
    }

    // components of peer identification
    #[derive(PartialEq, Clone, Copy)]
    pub struct IpAddress(u8, u8, u8, u8);

    #[derive(PartialEq)]
    pub struct Port(u16);

    #[derive(PartialEq)]
    pub struct PeerInfo {
        ip_address: IpAddress,
        port: Port,
        node_id: HashId,
    }

    #[derive(PartialEq)]
    struct PeerContact {
        peer_info: PeerInfo,
        seen: Instant,
    }

    pub struct KBucket {
        replication: u8,
        contacts: Vec<PeerContact>,
    }

    impl KBucket {
        pub fn new(k: u8) -> KBucket {
            KBucket {
                replication: k,
                contacts: Vec::new(),
            }
        }
        pub fn see_node(&mut self, peer_info: &PeerInfo) {
            // see if we know this node
            let mut index: usize = 0;
            for contact in self.contacts.iter() {
                if contact.peer_info == *peer_info {
                    break;
                }
                index += 1;
            }
            let node_is_known = index < self.contacts.len();

            // if so, its most recently seen now
            if node_is_known {
                self.move_contact_to_end(index);
                return;
            }

            // ping oldest node
            // match self.contacts.first() {
            //     Some(contact) => {}
            //     None => {}
            // };
        }
        fn move_contact_to_end(&mut self, index: usize) {
            let element = self.contacts.remove(index);
            self.contacts.push(element);
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        #[test]
        fn test() {
            let base_id = HashId([5324235, 0, 0, 0, 0, 0, 0, 0]);
            let other_id = HashId([3242578, 0, 0, 0, 0, 0, 0, 0]);

            let l_0 = base_id.leading_zeros(&other_id);
            assert_eq!(l_0, 9);
            assert_ne!(l_0, 256);

            let l_0 = base_id.leading_zeros(&base_id);
            assert_eq!(l_0, 256);
            assert_ne!(l_0, 9);
        }
    }

    pub struct DHT {
        replication: u8,
        my_peer_info: PeerInfo,
        k_buckets: HashMap<u8, KBucket>,
    }
    impl DHT {
        pub fn new(replication: u8, peer_info: PeerInfo) -> DHT {
            DHT {
                replication: replication,
                my_peer_info: peer_info,
                k_buckets: HashMap::new(),
            }
        }
        fn route_to_k_bucket(&mut self, peer_info: &PeerInfo) -> &mut KBucket {
            let my_id = self.my_peer_info.node_id;
            let other_id = peer_info.node_id;
            let bucket_index = my_id.leading_zeros(&other_id) as u8;
            self.k_buckets
                .entry(bucket_index)
                .or_insert(KBucket::new(self.replication))
        }
        fn see_node(&mut self, peer_info: PeerInfo) {
            let k_bucket = self.route_to_k_bucket(&peer_info);
            k_bucket.see_node(&peer_info);
        }
    }

    pub enum Message {
        Ping(PeerInfo),
        Store(PeerInfo),
        FindNode(PeerInfo, HashId),
        FindValue(PeerInfo, HashId),
    }
}
