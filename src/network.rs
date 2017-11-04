
use peer::{PeerID, Peer};
use std::collections::BTreeMap;
use multiaddr::Multiaddr;

pub struct RoutingTable {
    buckets: Vec<BTreeMap<PeerID, Peer>>,
}

impl RoutingTable {
    /// Creates new routing table with key_size+1 buckets (buckets 0 to key_size).
    /// All buckets are initialized empty
    pub fn new(key_size: usize) -> RoutingTable {
        let mut buckets = Vec::with_capacity(key_size);
        for _ in 0..key_size + 1 {
            buckets.push(BTreeMap::new());
        }

        RoutingTable { buckets }
    }

    fn _get_bucket(&self, bucket_number: usize) -> &BTreeMap<PeerID, Peer> {
        &(self.buckets.get(bucket_number).expect(
            "Bucket number out of range. The function for obtaining it must be wrong.",
        ))
    }

    pub fn get_peer(&self, id: &PeerID) -> Option<&Peer> {
        self._get_bucket(id.bucket_number()).get(id)
    }

    pub fn get_peer_or_neighbour(&self, id: &PeerID) -> Option<&Peer> {
        let bucket = self._get_bucket(id.bucket_number());
        let r = bucket.get(id);
        match r {
            Some(_) => r,
            None => {
                match bucket.iter().next() {
                    None => None,
                    Some((_, j)) => Some(j),
                }
            }
        }
    }

    pub fn update_or_insert_peer(&mut self, peer_id: PeerID, addr: Multiaddr) {
        unimplemented!();
    }
}
