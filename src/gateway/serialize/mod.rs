//! Module with build-in serialization methods.

pub mod binary;

use multiaddr::Multiaddr;
use id::UID;
use msg::{Msg, MsgType};


/// Struct for Msg, that is more directly serializable and supports convertion between it and `Msg`
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
            msg_id: UID::from(self.msg_id),
            peer_id: UID::from(self.peer_id),
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
    fn from(_err: ::multiaddr::Error) -> Error {
        Error::ParsingFailed
    }
}
