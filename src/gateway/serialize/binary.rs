use msg::Msg;
use multiaddr::Multiaddr;
use gateway::serialize::SerializableMsg;
use gateway::serialize::Error;


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




pub fn serialize(msg: Msg) -> BinMsg {
    let addr = (&msg).addr.clone();
    let smsg = SerializableMsg::from(msg);
    let bytes = ::bincode::serialize(&smsg, ::bincode::Infinite).unwrap();
    BinMsg::new(bytes, addr)
}

pub fn parse(binmsg: BinMsg) -> Result<Msg, Error> {
    let bytes = binmsg.payload;
    let smsg = ::bincode::deserialize::<SerializableMsg>(&bytes);
    let result = if let Ok(smsg) = smsg {
        smsg.to_msg(binmsg.addr)?
    } else {
        return Err(Error::ParsingFailed);
    };

    // TO BE CONSIDERED
    // maybe we should check if sender in content is the one who it was received from (blocks relaying and spoofing)
    Ok(result)
}
