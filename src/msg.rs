use multiaddr::{Multiaddr, ToMultiaddr};
use peer::{Peer, PeerID};
use rand::{OsRng,Rng};

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub struct MsgID {
    pub id: Vec<u8>,
}

impl From<Vec<u8>> for MsgID {
    fn from(id: Vec<u8>) -> MsgID {
        MsgID{ id }
    }
}

impl MsgID {
    pub fn random(size: usize) -> MsgID {
        let mut rng = OsRng::new().expect("Failed to initialize system random number generator.");
        let mut msg_id = Vec::with_capacity(size);
        msg_id.resize(size, 0);
        rng.fill_bytes(&mut msg_id);
        MsgID::from(msg_id)
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.id
    }
}

#[derive(Debug)]
pub struct Msg {
    pub msg_id: MsgID,
    pub peer_id: PeerID,
    pub addr: Multiaddr,
    pub msg_type: MsgType,
}

impl Msg {
    pub fn new_ping(mid: &PeerID, dst: &Peer) -> Msg {
        Msg {
            msg_id: MsgID::random(mid.len()),
            peer_id: mid.clone(),
            addr: dst.addr.clone(),
            msg_type: MsgType::REQ_PING,
        }
    }
    pub fn new_pong(mid: &PeerID, msg_id: MsgID, dst: &Multiaddr) -> Msg {
        Msg {
            msg_id: MsgID::random(mid.len()),
            addr: dst.clone(),
            peer_id: mid.clone(),
            msg_type: MsgType::RES_PONG,
        }
    }

    pub fn new_find_node(mid: &PeerID, dst: &Multiaddr, find_peer: &PeerID) -> Msg {
        Msg {
            msg_id: MsgID::random(mid.len()),
            addr: dst.clone(),
            peer_id: mid.clone(),
            msg_type: MsgType::REQ_FIND_NODE { peer_id: find_peer.clone() },
        }
    }

    pub fn new_value_found(mid: &PeerID,
        msg_id: MsgID ,dst: &Multiaddr,key: &Vec<u8>,value: &Vec<u8>) -> Msg {
        Msg {
            msg_id: msg_id,
            peer_id: mid.clone(),
            addr: dst.clone(),
            msg_type: MsgType::RES_VALUE_FOUND {
                key: key.clone(),
                value: value.clone(),
            },
        }
    }

    pub fn new_list_peers(mid: &PeerID, msg_id: MsgID,dst: &Multiaddr,peers: Vec<MsgPeer>,) -> Msg {
        Msg {
            peer_id: mid.clone(),
            msg_id: msg_id,
            addr: dst.clone(),
            msg_type: MsgType::RES_LIST_PEERS { peers },
        }
    }

    pub fn new_find_value(mid: &PeerID, dst: &Multiaddr, find_value: &Vec<u8>) -> Msg {
        Msg {
            peer_id: mid.clone(),
            msg_id: MsgID::random(mid.len()),
            addr: dst.clone(),
            msg_type: MsgType::REQ_FIND_VALUE { key: find_value.clone() },
        }
    }

    pub fn new_store(mid: &PeerID, dst: &Multiaddr, key: &Vec<u8>, value: &Vec<u8>) -> Msg {
        Msg {
            peer_id: mid.clone(),
            msg_id: MsgID::random(mid.len()),
            addr: dst.clone(),
            msg_type: MsgType::REQ_STORE {
                key: key.clone(),
                value: value.clone(),
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MsgType {
    REQ_PING,
    REQ_STORE { key: Vec<u8>, value: Vec<u8> },
    REQ_FIND_NODE { peer_id: PeerID },
    REQ_FIND_VALUE { key: Vec<u8> },
    RES_PONG,
    RES_LIST_PEERS { peers: Vec<MsgPeer> },
    RES_VALUE_FOUND { key: Vec<u8>, value: Vec<u8> },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgPeer {
    addr: String,
    peer_id: Vec<u8>,
}

impl From<Peer> for MsgPeer {
    fn from(peer: Peer) -> MsgPeer {
        MsgPeer {
            addr: peer.addr.to_string(),
            peer_id: peer.peer_id.owned_bytes(),
        }
    }
}

impl MsgPeer {
    pub fn new(peer: &Peer) -> MsgPeer {
        MsgPeer {
            addr: peer.addr.to_string(),
            peer_id: peer.peer_id.bytes().clone(),
        }
    }

    pub fn to_peer(self) -> Option<Peer> {
        let addr = self.addr.to_multiaddr();
        if let Ok(addr) = addr {
            Some(Peer::new(PeerID::from(self.peer_id), addr))
        } else {
            None
        }
    }
}
