
use peer::Peer;
use multiaddr::Multiaddr;
use std::time::Duration;
use std::slice::Iter;
use id::UID;

pub struct RoutingTable {
    peer_timeout: Duration,
    my_peer_id: UID,

    buckets: Vec<KBucket>,
}

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
    pub fn insert(&mut self, peer: Peer) -> Result<(),(Peer,Peer)> {
        if self.is_full() {
            Err((peer, self.peers.first().unwrap().clone()))
        } else {
            self.peers.push(peer);
            Ok(())
        }
    }

    pub fn insert_force(&mut self, peer: Peer) -> Option<Peer> {
        let mut r = None;
        if self.is_full() {
            r = self.peers.pop();
        }
        self.peers.push(peer);
        return r;
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
        for (i,v) in self.peers.iter().enumerate() {
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
}

impl RoutingTable {
    /// Creates new routing table with key_size+1 buckets (buckets 0 to key_size).
    /// All buckets are initialized empty
    pub fn new(my_peer_id: UID, bucket_size: usize, peer_timeout: Duration) -> RoutingTable {
        let mut buckets = Vec::with_capacity(my_peer_id.len());
        for i in 0..my_peer_id.len() {
            buckets.push(KBucket::new(bucket_size));
        }

        RoutingTable {
            my_peer_id: my_peer_id,
            buckets: buckets,
            peer_timeout: peer_timeout,
        }
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

    pub fn insert_peer(&mut self, peer: Peer) -> Result<(),(Peer, Peer)>{
        let mid = self.my_peer_id.clone();
        self.get_bucket_mut((&peer).peer_id.distance(&mid).bucket_number())
            .insert(peer)
    }

    pub fn get_peer(&self, id: &UID) -> Option<&Peer> {
        self.get_bucket(id.distance(&self.my_peer_id).bucket_number()).get(id)
    }

    pub fn remove_peer(&mut self, id: &UID) -> Option<Peer> {
        self.get_bucket_mut(id.bucket_number()).remove(id)
    }

    pub fn update_peer(&mut self, id: &UID) -> bool {
        let p = self.remove_peer(id);
        if let Some(mut peer) = p {
            peer.update_last_seen_timestamp();
            self.insert_peer(peer).unwrap();
            true
        } else {
            false
        }
    }

    pub fn get_k_nearest_peers(&self, peer_id: &UID, k: usize) -> Vec<Peer> {
        let mut result: Vec<Peer> = Vec::new();
        let bn = peer_id.distance(&self.my_peer_id).bucket_number() as isize;
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

            // break when we have enough peers from both sides of the buckets or if we run out of buckets
            if (out_of_range > 1) || (result.len() > k && i % 2 == 0) {
                break;
            }
        }

        result.sort_unstable_by(|a, b| {
            a.peer_id.distance(&peer_id).cmp(
                &(b.peer_id.distance(peer_id)),
            )
        });

        result.into_iter().enumerate().filter(|x| x.0 < k).map(|(_,x)| x).collect()
    }

    pub fn update_or_insert_peer(&mut self, peer_id: UID, addr: Multiaddr) -> Result<(),(Peer, Peer)> {
        let peer = self.remove_peer(&peer_id);
        if let Some(mut peer) = peer {
            peer.update_last_seen_timestamp();
            return self.insert_peer(peer);
        } else {
            let peer = Peer::new(peer_id, addr);
            return self.insert_peer(peer);
        }
    }

    pub fn get_old_peers(&mut self) -> Vec<Peer> {
        let mut old = Vec::new();
        for bucket in self.buckets.iter() {
            for peer in bucket.iter() {
                if peer.is_older_than(&self.peer_timeout) {
                    old.push(self.get_peer(&peer.peer_id).unwrap().clone());
                }
            }
        }

        old
    }

    fn list_not_full_buckets(&self) -> Vec<usize> {
        let mut result = Vec::new();
        for (i, bucket) in (&self.buckets).iter().enumerate() {
            if ! bucket.is_full() {
                result.push(i);
            }
        }

        result
    }

    pub fn is_full(&self) -> bool {
        self.list_not_full_buckets().len() > 0
    }

    pub fn len(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }
}
