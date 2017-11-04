extern crate multiaddr;
extern crate rand;
extern crate crypto;

pub mod hash;
pub mod gateway;

mod msg;
mod network;
mod peer;

use std::collections::HashMap;
use network::RoutingTable;
use rand::{OsRng, Rng};
use hash::DHTHasher;
use gateway::{MsgGateway, SendAbilityChecker};
use msg::{Msg, BinMsg, MsgType};
use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use multiaddr::Multiaddr;
use gateway::ControlMsg;
use peer::PeerID;

pub struct DHTService<T: DHTHasher> {
    hasher: T,
    gateways: Vec<Box<MsgGateway + Send>>,
    control_thread: Option<(Sender<DHTControlMsg>, JoinHandle<DHTManagement>)>,
    node_operation_data: Option<DHTManagement>,
}

struct RunningGateway {
    pub command_sender: Sender<ControlMsg>,
    pub handle: JoinHandle<()>,
    pub send_validator: Box<SendAbilityChecker + Send>,
}

struct DHTManagement {
    pub peer_id: PeerID,
    pub data_store: HashMap<Vec<u8>, Vec<u8>>,
    pub routing_table: RoutingTable,
    pub running_gateways: Vec<RunningGateway>,
    pub address: Multiaddr,
}

impl DHTManagement {
    fn send_msg(&self, msg: Msg) -> bool {
        //TODO implement round robin
        for (i, rgw) in self.running_gateways.iter().enumerate() {
            if rgw.send_validator.can_send(&msg.dest_addr) {
                // send it
                let s = rgw.command_sender.send(ControlMsg::SendMsg(msg));
                return s.is_ok();
            }
        }
        eprintln!("Can't send {:?}", msg);
        false
    }
}

/// This is public interface for this library
impl<T: DHTHasher> DHTService<T> {
    /// This will create new DHT service with specified DHTHasher. The node will generate a
    ///  random value, hash it with the hasher and that will be the node's ID. The lenght of
    ///  the resulting hash will also specify the keysize for the DHT
    pub fn new(hasher: T) -> DHTService<T> {
        // generate random peerID with correct size
        let mut rng = OsRng::new().expect("Failed to initialize system random number generator.");
        let mut peer_id = Vec::with_capacity(T::get_hash_bytes_count());
        peer_id.resize(T::get_hash_bytes_count(), 0);
        rng.fill_bytes(&mut peer_id);

        DHTService {
            hasher: hasher,
            gateways: Vec::new(),
            node_operation_data: Some(DHTManagement {
                running_gateways: Vec::new(),
                peer_id: PeerID::from(&peer_id),
                data_store: HashMap::new(),
                routing_table: RoutingTable::new(T::get_hash_bytes_count()),
                address: Multiaddr::new("").unwrap(),
            }),
            control_thread: None,
        }
    }
    /// Query value by key from the network, will return None when no value is found. When
    ///  the node is not connected to any other, it will answer based on local storage.
    pub fn query(&self, key: &Vec<u8>) -> Option<Vec<u8>> {
        unimplemented!();
    }
    /// Save value to the network, when not connected, it will be stored locally.
    pub fn save(&mut self, key: &Vec<u8>, data: &Vec<u8>) {
        let key = T::hash(key);
        let value = data.clone();
        if self.is_running() {
            //TODO send value to the management thread
            unimplemented!();
        } else {
            panic!("You should not save data before starting the DHT.")
        }
    }
    /// Start threads, which will handle communication with the rest of the DHT network
    pub fn start(&mut self, seed: Multiaddr) {
        if self.gateways.len() == 0 {
            panic!("Can't start DHTService with no gateways...");
        }

        let (ch_send, gw_receive): (Sender<Msg>, Receiver<Msg>) = mpsc::channel();

        // start all gateway threads
        let mut iter = self.gateways.drain(..);
        while let Some(mut gw) = iter.next() {
            let ch_send = ch_send.clone();
            let (control_send, control_receive): (Sender<ControlMsg>,
                                                  Receiver<ControlMsg>) = mpsc::channel();
            let validator = gw.get_send_ability_checker();

            let handle = thread::spawn(move || { gw.run(ch_send, control_receive); });
            self.node_operation_data
                .as_mut()
                .expect(
                    "Unexpected non-existance of DHT data. This should never happen.",
                )
                .running_gateways
                .push(RunningGateway {
                    command_sender: control_send,
                    handle: handle,
                    send_validator: validator,
                });
        }

        // start main management and processing thread
        let (control_channel_send, control_channel_recv): (Sender<DHTControlMsg>,
                                                           Receiver<DHTControlMsg>) =
            mpsc::channel();
        let node = self.node_operation_data.take().expect(
            "Node operation data are not supplied.",
        );
        self.node_operation_data = None;
        let handle = thread::spawn(move || {
            dht_processing_thread_loop(gw_receive, control_channel_recv, node)
        });
        self.control_thread = Some((control_channel_send, handle));
    }

    pub fn is_running(&self) -> bool {
        self.control_thread.is_some()
    }

    /// Stop all service threads
    pub fn stop(&mut self) {
        // stop management thread and save its data
        let (control_channel, handle) = self.control_thread.take().expect(
            "Control thread not running",
        );
        self.control_thread = None;
        let _ = control_channel.send(DHTControlMsg::Stop);
        let data = handle.join();
        if let Ok(nod) = data {
            self.node_operation_data = Some(nod);
        }

        // stop and discard all connections to gateway threads
        for gw in self.node_operation_data
            .as_mut()
            .expect(
                "Unexpected non-existance of DHT data. This should never happen.",
            )
            .running_gateways
            .drain(..)
        {
            gw.command_sender.send(ControlMsg::Stop).expect(
                "Gateway stopped by itself...",
            );
        }
    }

    /// register gateway, which will be used to communicate
    pub fn register_gateway(&mut self, gw: Box<MsgGateway + Send>) {
        self.gateways.push(gw);
    }
}

impl<T: DHTHasher> Drop for DHTService<T> {
    fn drop(&mut self) {
        if self.is_running() {
            panic!("The DHTService MUST be stopped before being dropped!");
        }
    }
}



fn dht_processing_thread_loop(
    incoming_msg: Receiver<Msg>,
    control_channel: Receiver<DHTControlMsg>,
    mut node: DHTManagement,
) -> DHTManagement {
    loop {
        // Try to read any incoming message
        let received = incoming_msg.try_recv();
        if let Ok(msg) = received {
            match msg.msg_type {
                // PING request
                MsgType::REQ_PING{peer_id} => {
                    node.routing_table.update_or_insert_peer(peer_id, msg.source_addr);
                    node.send_msg(Msg::pong(&msg.source_addr, &node.peer_id, &node.address));
                },
                // PONG response
                MsgType::RES_PONG{peer_id} => {
                    node.routing_table.update_or_insert_peer(peer_id, msg.source_addr);
                }
            }
        }

        let maybe_command = control_channel.try_recv();
        if let Ok(cmd) = maybe_command {
            // process incoming command
            match cmd {
                DHTControlMsg::Stop => return node,
                DHTControlMsg::Save { key, value } => {
                    node.data_store.insert(key, value);
                    // TODO distribute the data further into the network
                }
            }
        }

        // Handle gateways diing
    }
}

enum DHTControlMsg {
    Stop,
    Save { key: Vec<u8>, value: Vec<u8> },
}
