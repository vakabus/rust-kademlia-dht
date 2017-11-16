extern crate multiaddr;
extern crate rand;
extern crate crypto;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate bincode;

pub mod hash;
pub mod gateway;

mod msg;
mod routing;
mod peer;
mod dht_control;


use dht_control::*;
use hash::DHTHasher;
use gateway::{MsgGateway, SendAbilityChecker};
use msg::{Msg, MsgType, MsgPeer};
use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use multiaddr::Multiaddr;
use gateway::ControlMsg;
use peer::PeerID;
use std::time::Duration;

pub struct DHTService<T: DHTHasher> {
    hasher: T,
    gateways: Vec<Box<MsgGateway + Send>>,
    control_thread: Option<(Sender<DHTControlMsg>, JoinHandle<DHTManagement>)>,
    node_operation_data: Option<DHTManagement>,
}

/// This is public interface for this library
impl<T: DHTHasher> DHTService<T> {
    /// This will create new DHT service with specified DHTHasher. The node will generate a
    ///  random value, hash it with the hasher and that will be the node's ID. The lenght of
    ///  the resulting hash will also specify the keysize for the DHT
    pub fn new(
        hasher: T,
        bucket_size: usize,
        concurrency_degree: usize,
        peer_timeout: Duration,
    ) -> DHTService<T> {
        DHTService {
            hasher: hasher,
            gateways: Vec::new(),
            node_operation_data: Some(DHTManagement::new(
                T::get_hash_bytes_count(),
                bucket_size,
                concurrency_degree,
                peer_timeout,
            )),
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
                .add_running_gateway(control_send, handle, validator);
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
            dht_control::run(gw_receive, control_channel_recv, node)
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
            .drain_running_gateways()
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
