
use peer::*;
use multiaddr::Multiaddr;
use std::time::Duration;
use std::slice::Iter;
use id::UID;

#[derive(Debug)]
pub struct RoutingTable {
    peer_timeout: Duration,
    my_peer_id: UID,

    buckets: Vec<KBucket>,
}

#[derive(Debug)]
pub struct KBucket {
    max_size: usize,

    /// First is the least-recently seen Peer, last is the most-recently seen Peer
    peers: Vec<Peer>,
}

impl KBucket {
    pub fn new(size: usize) -> KBucket {
        KBucket {
            max_size: size,
            peers: Vec::new(),
        }
    }

    /// returns tuple (new, old) if failed
    pub fn insert(&mut self, peer: Peer, peer_timeout: &Duration) -> Result<(), PeerPair> {
        // prevent inserting something which is already in there
        if self.has_peer(&peer.peer_id) {
            warn!("Trying to insert already known peer!");
            return Ok(());
        }

        if self.is_full() {
            if self.peers.first().unwrap().is_older_than(peer_timeout) {
                Err(PeerPair::new(self.peers.first().unwrap().clone(), peer))
            } else {
                Ok(())
            }
        } else {
            self.peers.push(peer);
            Ok(())
        }
    }

    pub fn get(&self, peer_id: &UID) -> Option<&Peer> {
        for p in self.peers.iter() {
            if p.peer_id == *peer_id {
                return Some(&p);
            }
        }
        None
    }

    pub fn iter(&self) -> Iter<Peer> {
        self.peers.iter()
    }

    pub fn remove(&mut self, peer_id: &UID) -> Option<Peer> {
        let mut indx = None;
        for (i, v) in self.peers.iter().enumerate() {
            if v.peer_id == *peer_id {
                indx = Some(i);
            }
        }
        if let Some(i) = indx {
            Some(self.peers.remove(i))
        } else {
            None
        }
    }

    pub fn is_full(&self) -> bool {
        self.peers.len() == self.max_size
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn has_peer(&self, id: &UID) -> bool {
        self.peers
            .iter()
            .filter(|x| x.peer_id == *id)
            .next()
            .is_some()
    }

    pub fn get_first(&self) -> Option<&Peer> {
        self.peers.first()
    }
}

impl RoutingTable {
    /// Creates new routing table with key_size+1 buckets (buckets 0 to key_size).
    /// All buckets are initialized empty
    pub fn new(my_peer_id: UID, bucket_size: usize, peer_timeout: Duration) -> RoutingTable {
        debug!(
            "Creating routing table bucket_size={}, my_peer_id={:?}",
            bucket_size,
            my_peer_id
        );
        let mut buckets = Vec::with_capacity(my_peer_id.len() * 8 + 1);
        for _i in 0..my_peer_id.len() * 8 + 1 {
            buckets.push(KBucket::new(bucket_size));
        }

        RoutingTable {
            my_peer_id: my_peer_id,
            buckets: buckets,
            peer_timeout: peer_timeout,
        }
    }

    pub fn get_bucket_number(&self, id: &UID) -> usize {
        self.my_peer_id.distance(id).bucket_number()
    }

    fn get_bucket(&self, bucket_number: usize) -> &KBucket {
        self.buckets.get(bucket_number).expect(
            "Bucket number out of range. The function for obtaining it must be wrong.",
        )
    }

    fn get_bucket_mut(&mut self, bucket_number: usize) -> &mut KBucket {
        self.buckets.get_mut(bucket_number).expect(
            "Bucket number out of range. The function for obtaining it must be wrong.",
        )
    }

    pub fn insert_peer(&mut self, peer: Peer) -> Result<(), PeerPair> {
        // prevent inserting myself
        if self.my_peer_id == peer.peer_id {
            return Ok(());
        }

        let bn = self.get_bucket_number(&peer.peer_id);
        let timeout = &self.peer_timeout.clone();
        self.get_bucket_mut(bn).insert(peer, timeout)
    }

    pub fn get_peer(&self, id: &UID) -> Option<&Peer> {
        self.get_bucket(self.get_bucket_number(id)).get(id)
    }

    pub fn remove_peer(&mut self, id: &UID) -> Option<Peer> {
        let bn = self.get_bucket_number(id);
        self.get_bucket_mut(bn).remove(id)
    }

    pub fn update_peer(&mut self, id: &UID) -> bool {
        let p = self.remove_peer(id);
        if let Some(mut peer) = p {
            peer.update_last_seen_timestamp();
            let _ = self.insert_peer(peer); //we've just removed it, no need to check
            true
        } else {
            false
        }
    }

    pub fn get_k_nearest_peers(&self, peer_id: &UID, k: usize) -> Vec<Peer> {
        let mut result: Vec<Peer> = Vec::new();
        let bn = self.get_bucket_number(peer_id) as isize;
        let mut out_of_range = 0;
        for i in 1.. {
            let y: isize = (i / 2 as isize) * (-1 as isize).pow(i as u32);
            if (0 <= bn + y) && (bn + y < self.buckets.len() as isize) {
                out_of_range = 0;
                for peer in self.buckets.get((bn + y) as usize).unwrap().iter() {
                    result.push(peer.clone());
                }
            } else {
                out_of_range += 1;
            }

            // break when we have enough peers from both sides of the buckets
            // or if we run out of buckets
            if (out_of_range > 1) || (result.len() > k && i % 2 == 0) {
                break;
            }
        }

        result.sort_unstable_by(|a, b| {
            a.peer_id.distance(&peer_id).cmp(
                &(b.peer_id.distance(peer_id)),
            )
        });

        result
            .into_iter()
            .enumerate()
            .filter(|x| x.0 < k)
            .map(|(_, x)| x)
            .collect()
    }

    pub fn update_or_insert_peer(&mut self, peer_id: UID, addr: Multiaddr) -> Result<(), PeerPair> {
        let peer = self.remove_peer(&peer_id);
        if let Some(mut peer) = peer {
            peer.update_last_seen_timestamp();
            return self.insert_peer(peer);
        } else {
            let peer = Peer::new(peer_id, addr);
            return self.insert_peer(peer);
        }
    }

    pub fn len(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    pub fn has_peer(&self, id: &UID) -> bool {
        self.get_bucket(self.get_bucket_number(id)).has_peer(id)
    }

    pub fn get_bucket_most_recent(&self, bucket: usize) -> Option<&Peer> {
        self.get_bucket(bucket).get_first()
    }
}
