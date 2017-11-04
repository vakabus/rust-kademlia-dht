use multiaddr::Multiaddr;
use peer::{Peer,PeerID};

#[derive(Debug)]
pub struct Msg {
    pub id: Vec<u8>,
    pub dest_addr: Multiaddr,
    pub source_addr: Multiaddr,
    pub msg_type: MsgType,
}

impl Msg {
    fn generate_random_msg_id() -> Vec<u8> {
        unimplemented!();
    }

    pub fn ping(dst: Peer, me: Peer) -> Msg {
        Msg{
            id: Msg::generate_random_msg_id(),
            dest_addr: dst.addr.clone(),
            source_addr: me.addr.clone(),
            msg_type: MsgType::REQ_PING{peer_id: me.peer_id.clone() }
        }
    }
    pub fn pong(dst: &Multiaddr, my_peer_id: &PeerID, my_addr: &Multiaddr) -> Msg {
        Msg{
            id: Msg::generate_random_msg_id(),
            dest_addr: dst.clone(),
            source_addr: my_addr.clone(),
            msg_type: MsgType::RES_PONG{peer_id: my_peer_id.clone() }
        }
    }
}

#[derive(Debug)]
pub enum MsgType {
    REQ_PING { peer_id: PeerID },
    REQ_STORE { key: Vec<u8>, value: Vec<u8> },
    REQ_FIND_NODE { pper_id: PeerID },
    REQ_FIND_VALUE { key: Vec<u8> },
    RES_PONG { peer_id: PeerID },
    RES_LIST_PEERS { peers: Vec<PeerID> },
    RES_VALUE_FOUND { key: Vec<u8>, value: Vec<u8> },
}

#[derive(Debug)]
pub struct BinMsg {
    addr: Multiaddr,
    payload: Vec<u8>,
}

impl BinMsg {
    pub fn new(payload: Vec<u8>, addr: Multiaddr) -> BinMsg {
        BinMsg { addr, payload }
    }

    pub fn get_addr(&self) -> &Multiaddr {
        &self.addr
    }

    pub fn get_payload(&self) -> &Vec<u8> {
        &self.payload
    }
}