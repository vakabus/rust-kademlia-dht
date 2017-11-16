#[macro_escape]
use serde_derive;
//use serde::{Serialize,Deserialize};

pub mod binary;

use multiaddr::{ToMultiaddr, Multiaddr};
use peer::{Peer, PeerID};
use msg::{Msg, MsgType};

#[derive(Serialize, Deserialize)]
pub struct SerializableMsg {
    pub id: Vec<u8>,
    pub msg_type: MsgType,
}

impl From<Msg> for SerializableMsg {
    fn from(msg: Msg) -> SerializableMsg {
        SerializableMsg {
            id: msg.id,
            msg_type: msg.msg_type,
        }
    }
}

impl SerializableMsg {
    pub fn to_msg(self, addr: Multiaddr) -> Result<Msg, Error> {
        let msg = Msg {
            id: self.id,
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
