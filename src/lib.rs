//! This is crate is supposed to make running DHT's easy.
//!
//! # Design
//!
//! The DHT node in this library runs always in separate thread. The public API is non-blocking
//! except for `query`. When you interact with the client, you are communicating with
//! the management thread - the thread handling all the logic. The management thread processes all
//! incoming messages, reacts to them and it handles your request. But actual network
//! communication is done separately.
//!
//! The management thread runs alongside with gateway threads. Their only purpose is to handle
//! networking. There can be as many gateways as you wish and each one of them can communicate
//! in a different way, using different protocol and format. Build-in gateway is currently only
//! `UdpGateway`, which handles simple communication directly over UDP with straightforward
//! serialization format using `serde` crate. The only limitation in variaty of gateways is, that
//! the address must be representable using [multiaddr](https://github.com/multiformats/multiaddr).

extern crate multiaddr;
extern crate rand;
extern crate crypto;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate bincode;
#[macro_use]
extern crate log;

pub mod hash;
pub mod gateway;

mod msg;
mod routing;
mod peer;
mod dht_control;
mod id;


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
use std::time::Duration;
use dht_control::DHTControlMsg;
use id::UID;

pub struct DHTService<T: DHTHasher> {
    hasher: T,
    gateways: Vec<Box<MsgGateway + Send>>,
    control_thread: Option<ControlThread>,
    bucket_size: usize,
    concurrency_degree: usize,
    peer_timeout: Duration,
    msg_timeout: Duration,
}

struct ControlThread {
    sender: Sender<DHTControlMsg>,
    receiver: Receiver<(UID, Option<Vec<u8>>)>,
    handle: JoinHandle<DHTManagement>,
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
        msg_timeout: Duration,
    ) -> DHTService<T> {
        info!(
            "Initiating DHTService, hash_size={}",
            T::get_hash_bytes_count()
        );
        DHTService {
            hasher: hasher,
            gateways: Vec::new(),
            control_thread: None,
            bucket_size,
            concurrency_degree,
            peer_timeout,
            msg_timeout,
        }
    }



    /// Query value by key from the network, will return None when no value is found. When
    ///  the node is not connected to any other, it will answer based on local storage.
    pub fn query(&self, key: &Vec<u8>) -> Option<Vec<u8>> {
        assert!(self.is_running());

        // send request
        let hkey = UID::from(T::hash(key));
        let q = DHTControlMsg::Query { key: hkey.clone() };

        info!("Query for key={:?}, hashed={:?}", key, hkey);

        let res = self.control_thread
            .as_ref()
            .expect("When DHT is running, control thread should be available.")
            .sender
            .send(q);
        if let Err(err) = res {
            error!("Failed to send query to management thread.");
            return None;
        }

        //wait for response
        let res = self.control_thread
            .as_ref()
            .expect("When DHT is running, control thread should be available.")
            .receiver
            .recv();
        match res {
            Ok((rkey, value)) => {
                if rkey != hkey {
                    error!("Requested different key!");
                    return None;
                } else {
                    return value;
                }
            }
            Err(err) => {
                error!("Failed to receive response to query.");
                return None;
            }
        }
    }


    /// Save value to the network, when not connected, it will be stored locally.
    pub fn save(&mut self, key: &Vec<u8>, data: Vec<u8>) {
        assert!(self.is_running());

        let key = UID::from(T::hash(key));
        let value = data.clone();
        let s = DHTControlMsg::Save { key, value };
        let res = self.control_thread
            .as_ref()
            .expect("When DHT is running, control thread should be available.")
            .sender
            .send(s);

    }

    pub fn connect(&mut self, seed: Multiaddr) {
        assert!(self.is_running());

        info!("Adding new seed... ({:?})", seed);

        self.control_thread
            .as_ref()
            .expect("When DHT is running, control thread should be available.")
            .sender
            .send(DHTControlMsg::Connect { addr: seed });
    }



    /// Start threads, which will handle communication with the rest of the DHT network
    pub fn start(&mut self) {
        info!("Starting...");

        // error checks
        if self.gateways.len() == 0 {
            panic!("Can't start DHTService with no gateways...");
        }


        // Generate communication channels
        let (control_channel_send, control_channel_recv): (Sender<DHTControlMsg>,
                                                           Receiver<DHTControlMsg>) =
            mpsc::channel();
        let (response_channel_send, response_channler_recv): (Sender<
            (UID,
             Option<Vec<u8>>),
        >,
                                                              Receiver<
            (UID,
             Option<Vec<u8>>),
        >) = mpsc::channel();
        let (ch_send, gw_receive): (Sender<Msg>, Receiver<Msg>) = mpsc::channel();

        // create DHTManagement object
        let mut node = DHTManagement::new(
            T::get_hash_bytes_count(),
            self.bucket_size,
            self.concurrency_degree,
            self.peer_timeout,
            self.msg_timeout,
            gw_receive,
            control_channel_recv,
            response_channel_send,
        );

        // start all gateway threads
        let mut iter = self.gateways.drain(..);
        while let Some(mut gw) = iter.next() {
            let ch_send = ch_send.clone();
            let (control_send, control_receive): (Sender<ControlMsg>,
                                                  Receiver<ControlMsg>) = mpsc::channel();
            let validator = gw.get_send_ability_checker();

            let handle = thread::Builder::new().name("gateway".to_string()).spawn(
                move || { gw.run(ch_send, control_receive); },
            );
            let handle = handle.expect("Failed to spawn gateway thread.");
            node.add_running_gateway(control_send, handle, validator);
        }

        // start main management and processing thread
        let handle = thread::Builder::new()
            .name("management".to_string())
            .spawn(move || dht_control::run(node))
            .expect("Failed to spawn management thread;");
        self.control_thread = Some(ControlThread {
            sender: control_channel_send,
            handle: handle,
            receiver: response_channler_recv,
        });
    }


    pub fn is_running(&self) -> bool {
        self.control_thread.is_some()
    }



    /// Stop all service threads
    pub fn stop(&mut self) -> DHTManagement {
        // stop management thread and save its data
        let ControlThread {
            sender,
            receiver,
            handle,
        } = self.control_thread.take().expect(
            "Control thread not running",
        );
        self.control_thread = None;
        let _ = sender.send(DHTControlMsg::Stop);
        let mut data = handle.join().unwrap();

        // stop and discard all connections to gateway threads
        for gw in data.drain_running_gateways() {
            gw.command_sender.send(ControlMsg::Stop).expect(
                "Gateway stopped by itself...",
            );
        }

        data
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
