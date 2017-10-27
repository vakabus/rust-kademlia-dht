
use std::collections::BTreeMap;
use multiaddr::{Multiaddr,ToMultiaddr};

//#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Peer {
    peer_id: Vec<u8>,
    addr: Multiaddr,
    last_seen: u64
}

impl Peer {
    pub fn send_msg(payload: Vec<u8>) {
        unimplemented!();
    }

    pub fn update_last_seen_timestamp() {
        unimplemented!();
    }
}

pub struct RoutingTable {
    
}

impl RoutingTable {
    pub fn new() -> RoutingTable {
        unimplemented!();
    }

    pub fn get_peer(&self, peer_id: Vec<u8>) -> Option<Peer> {
        unimplemented!();
    }

    pub fn get_nearest(&self, peer_id: Vec<u8>) {
        unimplemented!();
    }
}