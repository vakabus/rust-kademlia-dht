extern crate multiaddr;
extern crate rand;

mod network;

use std::collections::HashMap;
use network::{RoutingTable, Peer};
use rand::{OsRng,Rng};

pub struct DHTService<T: DHTHasher> {
    hasher: T,
    peer_id: Vec<u8>,
    routing_table: RoutingTable,
    data_store: HashMap<Vec<u8>,Vec<u8>>
}

/// This is public interface for this library
impl <T: DHTHasher> DHTService<T> {
    /// This will create new DHT service with specified DHTHasher. The node will generate a random value, hash it with the hasher and that will be the node's ID. The lenght of the resulting hash will also specify the keysize for the DHT
    pub fn new(hasher: T) -> DHTService<T> {
        // generate random peerID with correct size
        let mut rng = OsRng::new().expect("Failed to initialize system random number generator.");
        let mut peer_id = Vec::with_capacity(T::get_hash_bytes_count());
        peer_id.resize(T::get_hash_bytes_count(), 0);
        rng.fill_bytes(&mut peer_id);

        DHTService{
            hasher: hasher,
            peer_id: peer_id,
            routing_table: RoutingTable::new(),
            data_store: HashMap::new()
        }
    }
    /// Query value by key from the network, will return None when no value is found. When the node is not connected to any other, it will answer based on local storage.
    pub fn query_value(&self, key: &Vec<u8>) -> Option<Vec<u8>> {
        unimplemented!();
    }
    /// Save value to the network, when not connected, it will be stored locally.
    pub fn save_value(&self, key: &Vec<u8>, data: &Vec<u8>) {
        unimplemented!();
    }
    /// Start a thread, which will handle communication with the rest of the DHT network
    pub fn start(&self, seed: Peer) {
        unimplemented!();
    }
    /// Stop the communication thread.
    pub fn stop(&self) {
        unimplemented!();
    }
}

pub trait DHTHasher {
    /// Hash any given data to an array of constant size.
    fn hash(data: &[u8]) -> Vec<u8>;
    fn get_hash_bytes_count() -> usize;
}
