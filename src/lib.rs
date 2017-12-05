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
//! 
//! This library provides best-effort access to DHT network. This is due to the nature of supported
//! network protocols - for example UDP. Queries can sometimes return nothing even though the value
//! exists somewhere in the network. This should happen only in cases, when the network connection
//! fails.
//! 
//! # Usage
//! 
//! To connect to DHT network, you need to instantiate a configured client, setup at least one
//! gateway, start the service. After that, you will need to know an address of at least one
//! other peer in network, so we can connect to it and seed our communication from him.
//! 
//! ```
//! extern crate multiaddr;
//! extern crate kademlia_dht;
//! 
//! use std::time::Duration;
//!     
//! use kademlia_dht::DHTService;
//! use kademlia_dht::hash::sha1;
//! use kademlia_dht::gateway::UdpGateway;
//! 
//! use multiaddr::ToMultiaddr;
//! 
//! fn main() {
//!     let mut service = DHTService::new(
//!         sha1::SHA1Hasher::new(),        // configure hashing algoritm used
//!         20,                             // bucket size 
//!                                         // (#hash bits * bucketnumber
//!                                         //       = routing table max size)
//!         3,                              // query concurrency value
//!         Duration::from_secs(15 * 60),   // peer timeout (for how long 
//!                                         //               we consider node active)
//!         Duration::from_secs(3),         // communication timeout
//!         Duration::from_secs(60 * 60),   // storage (cache) timeout
//!     );
//!     
//!     // at least one gateway is required
//!     service.register_gateway(UdpGateway::new("127.0.0.13:12345"));
//!     
//!     // start service (spawns threads)
//!     service.start();
//!     
//!     // connect to seed specified by Multiaddr (can be more than one)
//!     service.connect("/ip4/127.0.0.17/udp/12345".to_multiaddr().unwrap());
//!     
//!     // now the client knows about the network and it takes some time to initialize
//!     // at least the first connection
//! 
//!     // get number of known peers. When 0, the client is not connected to anywhere
//!     let n_peers = service.get_number_of_known_peers();
//!     
//!     // after a while, you can query
//!     let result = service.query(&vec![1u8, 8u8, 255u8]);
//!     
//!     // or save value to the network
//!     let result = service.save(&vec![1u8, 8u8, 255u8], vec![1u8, 8u8, 255u8, 254u8]);
//!     
//!     // and when you are done, you should stop the node, better to do it manually,
//!     // but when dropped, the DHTService will stop itself.
//!     service.stop();
//! }
//! ```
//! 
//! You can have as many clients as you want in single application. 

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
use gateway::MsgGateway;
use msg::Msg;
use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use multiaddr::Multiaddr;
use gateway::ControlMsg;
use std::time::Duration;
use id::UID;

pub struct DHTService<T: DHTHasher> {
    _hasher: T,
    gateways: Vec<Box<MsgGateway + Send>>,
    control_thread: Option<ControlThread>,
    bucket_size: usize,
    concurrency_degree: usize,
    peer_timeout: Duration,
    msg_timeout: Duration,
    storage_timeout: Duration,
}

struct ControlThread {
    sender: Sender<DHTControlMsg>,
    receiver: Receiver<DHTResponseMsg>,
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
        storage_timeout: Duration,
    ) -> DHTService<T> {
        info!(
            "Initiating DHTService, hash_size={}",
            T::get_hash_bytes_count()
        );
        DHTService {
            _hasher: hasher,
            gateways: Vec::new(),
            control_thread: None,
            bucket_size,
            concurrency_degree,
            peer_timeout,
            msg_timeout,
            storage_timeout,
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
        if let Err(_) = res {
            error!("Failed to send query to management thread.");
            return None;
        }

        //wait for response
        let res = self.control_thread
            .as_ref()
            .expect("When DHT is running, control thread should be available.")
            .receiver
            .recv();
        if res.is_err() {
            error!("Failed to receive response to query.");
            return None;
        }
        match res.unwrap() {
            DHTResponseMsg::QueryResponse{key, value} => {
                if key != hkey {
                    error!("Requested different key!");
                    return None;
                } else {
                    return value;
                }
            }
            _ => {
                error!("Received response to something else...");
                return None;
            }
        }
    }


    /// Retrieve number of peers in the routing table. Returns 0 when errors occur.
    pub fn get_number_of_known_peers(&self) -> usize {
        let res = self.control_thread.as_ref().expect("When DHT is running, control thread should be available.").sender.send(DHTControlMsg::GetNumberOfKnownPeers);
        if res.is_err() {
            error!("Couldn't send message to DHT client...");
            return 0;
        }

        let res = self.control_thread
            .as_ref()
            .expect("When DHT is running, control thread should be available.")
            .receiver
            .recv();

        if res.is_err() {
            error!("Couldn't receive message from DHT client...");
            return 0;
        }

        if let DHTResponseMsg::NumberOfKnownPeers{peers} = res.unwrap() {
            return peers;
        } else {
            error!("Received response to something else...");
            return 0;
        }
    }


    /// Save value to the network, when not connected, it will be stored locally.
    pub fn save(
        &mut self,
        key: &Vec<u8>,
        data: Vec<u8>,
    ) -> std::result::Result<(), std::sync::mpsc::SendError<dht_control::DHTControlMsg>> {
        assert!(self.is_running());

        info!("Saving data key={:?} data={:?}", key, data);

        let key = UID::from(T::hash(key));
        let value = data.clone();
        let s = DHTControlMsg::Save { key, value };
        self.control_thread
            .as_ref()
            .expect("When DHT is running, control thread should be available.")
            .sender
            .send(s)

    }

    pub fn connect(
        &mut self,
        seed: Multiaddr,
    ) -> std::result::Result<(), std::sync::mpsc::SendError<dht_control::DHTControlMsg>> {
        assert!(self.is_running());

        info!("Adding new seed... ({:?})", seed);

        self.control_thread
            .as_ref()
            .expect("When DHT is running, control thread should be available.")
            .sender
            .send(DHTControlMsg::Connect { addr: seed })
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
        let (response_channel_send, response_channler_recv): (Sender<DHTResponseMsg>,
                                                              Receiver<DHTResponseMsg>) = 
            mpsc::channel();
        let (ch_send, gw_receive): (Sender<Msg>, Receiver<Msg>) = mpsc::channel();

        // create DHTManagement object
        let mut node = DHTManagement::new(
            T::get_hash_bytes_count(),
            self.bucket_size,
            self.concurrency_degree,
            self.peer_timeout,
            self.msg_timeout,
            self.storage_timeout,
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
            receiver: _,
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
            self.stop();
            //panic!("The DHTService MUST be stopped before being dropped!");
        }
    }
}
