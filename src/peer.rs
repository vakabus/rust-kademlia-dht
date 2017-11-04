use std::cmp::Ordering;
use std::time::Instant;
use multiaddr::{Multiaddr, ToMultiaddr};

fn count_leading_zeros(b: u8) -> usize {
    let mut b = b;
    let mut i = 8;
    while b != 0 {
        i -= 1;
        b = b >> 1;
    }

    i
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PeerID {
    peer_id: Vec<u8>,
}

impl PeerID {
    pub fn from(id: &Vec<u8>) -> PeerID {
        PeerID { peer_id: id.clone() }
    }

    pub fn bucket_number(&self) -> usize {
        for (i, v) in self.peer_id.iter().enumerate() {
            if *v > 0 {
                return i * 8 + count_leading_zeros(*v);
            }
        }

        self.peer_id.len() * 8
    }
}

impl Clone for PeerID {
    fn clone(&self) -> PeerID {
        PeerID { peer_id: self.peer_id.clone() }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Peer {
    pub peer_id: PeerID,
    pub addr: Multiaddr,
    pub last_seen_timestamp: Instant,
}

impl Peer {
    pub fn new(peer_id: &PeerID, addr: &Multiaddr) -> Peer {
        Peer {
            peer_id: peer_id.clone(),
            addr: addr.clone(),
            last_seen_timestamp: Instant::now(),
        }
    }

    pub fn update_last_seen_timestamp(&mut self) {
        self.last_seen_timestamp = Instant::now();
    }

    pub fn get_last_time_seen(&self) -> &Instant {
        &self.last_seen_timestamp
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
        let p = PeerID::from(&(vec![0, 0, 0, 0]));
        assert_eq!(p.bucket_number(), 32);

        let p = PeerID::from(&(vec![0, 127, 0, 0]));
        assert_eq!(p.bucket_number(), 9);

        let p = PeerID::from(&(vec![0, 0, 1, 0]));
        assert_eq!(p.bucket_number(), 23);

        let p = PeerID::from(&(vec![0, 0, 0, 1]));
        assert_eq!(p.bucket_number(), 31);

        let p = PeerID::from(&(vec![255, 0, 0, 0]));
        assert_eq!(p.bucket_number(), 0);
    }
}
