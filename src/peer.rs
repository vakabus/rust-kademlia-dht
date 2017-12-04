use std::cmp::Ordering;
use std::time::{Duration, Instant};
use multiaddr::Multiaddr;
use id::UID;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Peer {
    pub peer_id: UID,
    pub addr: Multiaddr,
    pub last_seen_timestamp: Instant,
}

impl Peer {
    pub fn new(peer_id: UID, addr: Multiaddr) -> Peer {
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

pub struct PeerPair {
    pub old: Peer,
    pub new: Peer,
}

impl PeerPair {
    pub fn new(old: Peer, new: Peer) -> PeerPair {
        PeerPair { old, new }
    }
}
