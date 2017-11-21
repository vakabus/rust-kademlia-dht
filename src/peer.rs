use std::cmp::Ordering;
use std::time::{Duration, Instant};
use multiaddr::{Multiaddr, ToMultiaddr};
use rand::{OsRng, Rng};

fn count_leading_zeros(b: u8) -> usize {
    let mut b = b;
    let mut i = 8;
    while b != 0 {
        i -= 1;
        b = b >> 1;
    }

    i
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, Hash)]
pub struct PeerID {
    peer_id: Vec<u8>,
}


/// PeerID is little-endian reprezentation of node's ID. Saved in an array of bytes.
impl PeerID {
    pub fn new(id: &Vec<u8>) -> PeerID {
        PeerID { peer_id: id.clone() }
    }

    pub fn random(size: usize) -> PeerID {
        let mut rng = OsRng::new().expect("Failed to initialize system random number generator.");
        let mut peer_id = Vec::with_capacity(size);
        peer_id.resize(size, 0);
        rng.fill_bytes(&mut peer_id);
        PeerID::from(peer_id)
    }

    pub fn random_within_bucket(size: usize, leading_zeros: usize) -> PeerID {
        if size < leading_zeros {
            panic!("Can't generate more leading zeros than the length of the ID.");
        };

        let mut random = PeerID::random(size);
        let mut i = 0;
        let mut leading_zeros = leading_zeros;
        while leading_zeros >= 8 {
            random.peer_id[i] = 0;
            leading_zeros -= 8;
            i += 1;
        }
        random.peer_id[i] = (random.peer_id[i] & (255u8 >> leading_zeros)) | (1 << (7-leading_zeros));

        random
    }

    pub fn bucket_number(&self) -> usize {
        for (i, v) in self.peer_id.iter().enumerate() {
            if *v > 0 {
                return i * 8 + count_leading_zeros(*v);
            }
        }

        self.peer_id.len() * 8
    }

    pub fn distance(&self, other: &PeerID) -> PeerID {
        assert_eq!(self.peer_id.len(), other.peer_id.len());

        let res: Vec<u8> = (&self.peer_id)
            .iter()
            .zip((&other.peer_id).iter())
            .map(|(x, y)| x ^ y)
            .collect();
        PeerID::from(res)
    }

    pub fn len(&self) -> usize {
        self.peer_id.len()
    }

    pub fn bit_at(&self, pos: usize) -> bool {
        assert!(pos > self.peer_id.len()*8);

        let b = pos / 8;
        let c = pos % 8;
    
        self.peer_id[b] & (1 << c) > 0
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.peer_id
    }

    pub fn owned_bytes(self) -> Vec<u8> {
        self.peer_id
    }
}

impl Clone for PeerID {
    fn clone(&self) -> PeerID {
        PeerID { peer_id: self.peer_id.clone() }
    }
}

impl From<Vec<u8>> for PeerID {
    fn from(bytes: Vec<u8>) -> PeerID {
        PeerID { peer_id: bytes }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Peer {
    pub peer_id: PeerID,
    pub addr: Multiaddr,
    pub last_seen_timestamp: Instant,
}

impl Peer {
    pub fn new(peer_id: PeerID, addr: Multiaddr) -> Peer {
        Peer {
            peer_id: peer_id,
            addr: addr,
            last_seen_timestamp: Instant::now(),
        }
    }

    pub fn update_last_seen_timestamp(&mut self) {
        self.last_seen_timestamp = Instant::now();
    }

    pub fn get_last_time_seen(&self) -> &Instant {
        &self.last_seen_timestamp
    }

    pub fn is_older_than(&self, max_age: &Duration) -> bool {
        self.last_seen_timestamp.elapsed() > *max_age
    }
}

/// Peers are ordered lexicographically by last_seen_timestamp and then by peer_id.
impl PartialOrd for Peer {
    fn partial_cmp(&self, other: &Peer) -> Option<Ordering> {
        let ord = self.last_seen_timestamp.cmp(&other.last_seen_timestamp);
        match ord {
            Ordering::Equal => Some(self.peer_id.cmp(&other.peer_id)),
            _ => Some(ord),
        }
    }
}

impl Ord for Peer {
    fn cmp(&self, other: &Peer) -> Ordering {
        self.partial_cmp(&other).expect(
            "It should be impossible to not have non-comparable Peers",
        )
    }
}


#[cfg(test)]
mod tests {

    use peer::count_leading_zeros;
    use peer::PeerID;

    #[test]
    fn test_count_leading_zeros() {
        assert_eq!(count_leading_zeros(0), 8);
        assert_eq!(count_leading_zeros(1), 7);
        assert_eq!(count_leading_zeros(2), 6);
        assert_eq!(count_leading_zeros(4), 5);
        assert_eq!(count_leading_zeros(8), 4);
        assert_eq!(count_leading_zeros(16), 3);
        assert_eq!(count_leading_zeros(32), 2);
        assert_eq!(count_leading_zeros(64), 1);
        assert_eq!(count_leading_zeros(128), 0);
    }

    #[test]
    fn test_peer_id_bucket() {
        let p = PeerID::from(Vec::<u8>::from(vec![0, 0, 0, 0]));
        assert_eq!(p.bucket_number(), 32);

        let p = PeerID::from(vec![0, 127, 0, 0]);
        assert_eq!(p.bucket_number(), 9);

        let p = PeerID::from(vec![0, 0, 1, 0]);
        assert_eq!(p.bucket_number(), 23);

        let p = PeerID::from(vec![0, 0, 0, 1]);
        assert_eq!(p.bucket_number(), 31);

        let p = PeerID::from(vec![255, 0, 0, 0]);
        assert_eq!(p.bucket_number(), 0);
    }

    #[test]
    fn test_random_generation() {
        let p = PeerID::random_within_bucket(20, 15);
        assert_eq!(15, p.bucket_number());

        let p = PeerID::random_within_bucket(20, 3);
        assert_eq!(3, p.bucket_number());
    }
}
