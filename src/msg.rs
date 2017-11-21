use multiaddr::{Multiaddr, ToMultiaddr};
use peer::Peer;
use id::UID;



#[derive(Debug)]
pub struct Msg {
    pub msg_id: UID,
    pub peer_id: UID,
    pub addr: Multiaddr,
    pub msg_type: MsgType,
}

impl Msg {
    pub fn new_ping(mid: &UID, dst: &Peer) -> Msg {
        Msg {
            msg_id: UID::random(mid.len()),
            peer_id: mid.clone(),
            addr: dst.addr.clone(),
            msg_type: MsgType::REQ_PING,
        }
    }
    pub fn new_pong(mid: &UID, msg_id: UID, dst: &Multiaddr) -> Msg {
        Msg {
            msg_id: UID::random(mid.len()),
            addr: dst.clone(),
            peer_id: mid.clone(),
            msg_type: MsgType::RES_PONG,
        }
    }

    pub fn new_find_node(mid: &UID, dst: &Multiaddr, find_peer: &UID) -> Msg {
        Msg {
            msg_id: UID::random(mid.len()),
            addr: dst.clone(),
            peer_id: mid.clone(),
            msg_type: MsgType::REQ_FIND_NODE { peer_id: find_peer.clone() },
        }
    }

    pub fn new_value_found(mid: &UID,
        msg_id: UID ,dst: &Multiaddr,key: UID,value: &Vec<u8>) -> Msg {
        Msg {
            msg_id: msg_id,
            peer_id: mid.clone(),
            addr: dst.clone(),
            msg_type: MsgType::RES_VALUE_FOUND {
                key: key,
                value: value.clone(),
            },
        }
    }

    pub fn new_list_peers(mid: &UID, msg_id: UID,dst: &Multiaddr,peers: Vec<MsgPeer>,) -> Msg {
        Msg {
            peer_id: mid.clone(),
            msg_id: msg_id,
            addr: dst.clone(),
            msg_type: MsgType::RES_LIST_PEERS { peers },
        }
    }

    pub fn new_find_value(mid: &UID, dst: &Multiaddr, find_value: &UID) -> Msg {
        Msg {
            peer_id: mid.clone(),
            msg_id: UID::random(mid.len()),
            addr: dst.clone(),
            msg_type: MsgType::REQ_FIND_VALUE { key: find_value.clone() },
        }
    }

    pub fn new_store(mid: &UID, dst: &Multiaddr, key: UID, value: &Vec<u8>) -> Msg {
        Msg {
            peer_id: mid.clone(),
            msg_id: UID::random(mid.len()),
            addr: dst.clone(),
            msg_type: MsgType::REQ_STORE {
                key: key,
                value: value.clone(),
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MsgType {
    REQ_PING,
    REQ_STORE { key: UID, value: Vec<u8> },
    REQ_FIND_NODE { peer_id: UID },
    REQ_FIND_VALUE { key: UID },
    RES_PONG,
    RES_LIST_PEERS { peers: Vec<MsgPeer> },
    RES_VALUE_FOUND { key: UID, value: Vec<u8> },
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
            Some(Peer::new(UID::from(self.peer_id), addr))
        } else {
            None
        }
    }
}
