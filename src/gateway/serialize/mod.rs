#[macro_escape]
use serde_derive;
//use serde::{Serialize,Deserialize};

pub mod binary;

use multiaddr::{ToMultiaddr, Multiaddr};
use peer::{Peer, PeerID};
use msg::{Msg, MsgType, MsgID};

#[derive(Serialize, Deserialize)]
pub struct SerializableMsg {
    pub msg_id: Vec<u8>,
    pub peer_id: Vec<u8>,
    pub msg_type: MsgType,
}

impl From<Msg> for SerializableMsg {
    fn from(msg: Msg) -> SerializableMsg {
        SerializableMsg {
            peer_id: msg.peer_id.bytes().clone(),
            msg_id: msg.msg_id.bytes().clone(),
            msg_type: msg.msg_type,
        }
    }
}

impl SerializableMsg {
    pub fn to_msg(self, addr: Multiaddr) -> Result<Msg, Error> {
        let msg = Msg {
            msg_id: MsgID::from(self.msg_id),
            peer_id: PeerID::from(self.peer_id),
            msg_type: self.msg_type,
            addr: addr,
        };
        Ok(msg)
    }
}

pub enum Error {
    SerializationFailed,
    ParsingFailed,
}

impl From<::multiaddr::Error> for Error {
    fn from(err: ::multiaddr::Error) -> Error {
        Error::ParsingFailed
    }
}
