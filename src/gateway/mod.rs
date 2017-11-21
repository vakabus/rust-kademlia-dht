//! The DHT is designed to be able to work with network interface of any kind. To handle that, it
//! introduces concept of message gateways. Each gateway specifies to what kind of network
//! addresses it can send data. When communicating, the main DHT management than chooses, which
//! gateway is most appropriate for communicating with other peer.
//!
//! In the most basic form, each gateway runs in its own thread. This way, if you want to support
//! network interfaces, which are slow and/or in any other way problematic, they won't interfere
//! with the rest of the system. However, if you don't want to use so many threads, you can merge
//! your gateways using special MergerGateway. All the "subgateways" contained in it will run in
//! the same thread and they will take turns in receiving data.
//!
//! Gateways also take care of data serialization. It means, that for every possible network
//! channel, the most appropriate format can be chosen. For example, communication with JS clients
//! over WebSocket is best handled through JSON, but communication through UDP is better in some
//! binary format. This also makes it possible to create gateways, which could be used to
//! communicate with existing networks. For example with BitTorrent DHT.

mod udp;
pub use gateway::udp::UdpGateway;
mod merger;
pub use gateway::merger::MergerGateway;


pub mod serialize;

use multiaddr::Multiaddr;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use msg::Msg;

pub enum ControlMsg {
    Stop,
    SendMsg(Msg),
}



pub trait MsgGateway {
    /// does not block, returns Some(Msg) when received
    fn recv(&mut self) -> Option<Msg>;
    /// Sends msg to the recipient, returns True if succeeded
    fn send(&mut self, msg: Msg) -> bool;
    /// Returns address of the gateway, must be known before calling first recv()
    fn get_address(&self) -> Multiaddr;

    fn get_send_ability_checker(&self) -> Box<SendAbilityChecker + Send>;

    fn run(&mut self, msg_channel: Sender<Msg>, command_channel: Receiver<ControlMsg>) {
        loop {
            // handle receiving
            let m = self.recv();
            if let Some(msg) = m {
                msg_channel.send(msg);
            }

            let control_msg = command_channel.try_recv();
            if let Ok(msg) = control_msg {
                match msg {
                    ControlMsg::Stop => break,
                    ControlMsg::SendMsg(m) => {
                        let _ = self.send(m);
                    }
                }
            }
        }
    }
}


pub trait SendAbilityChecker {
    fn can_send(&self, maddr: &Multiaddr) -> bool;
}

pub struct ProtocolBasedSendAbilityChecker {
    maddr: Multiaddr,
}

impl ProtocolBasedSendAbilityChecker {
    pub fn new(maddr: Multiaddr) -> ProtocolBasedSendAbilityChecker {
        ProtocolBasedSendAbilityChecker { maddr }
    }
}

impl SendAbilityChecker for ProtocolBasedSendAbilityChecker {
    fn can_send(&self, maddr: &Multiaddr) -> bool {
        maddr.protocol() == self.maddr.protocol()
    }
}
