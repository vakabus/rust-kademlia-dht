extern crate kademlia_dht;
extern crate multiaddr;
extern crate env_logger;

use kademlia_dht::DHTService;
use kademlia_dht::hash::sha1;
use kademlia_dht::gateway::UdpGateway;
use kademlia_dht::hash::DHTHasher;

use std::thread;
use std::time::Duration;
use multiaddr::*;


use std::sync::{Once, ONCE_INIT};
static INIT: Once = ONCE_INIT;

/// Setup function that is only run once, even if called multiple times.
fn init_logger() {
    INIT.call_once(|| { env_logger::init().unwrap(); });
}


#[test]
fn start_and_stop() {
    // init logger
    init_logger();

    let mut service = new_udp_sha1_node("127.0.0.1:12345");

    //start
    service.start();

    //wait
    thread::sleep(Duration::from_secs(3));

    //stop
    service.stop();
}

#[test]
fn two_node_network() {
    // init logger
    init_logger();

    // init nodes
    let mut node1 = new_udp_sha1_node("127.0.0.2:12345");
    let mut node2 = new_udp_sha1_node("127.0.0.3:12345");

    // start nodes
    node1.start();
    node2.start();

    let _ = node1.save(&(b"key".to_vec()), (b"value".to_vec()));
    let _ = node2.connect("/ip4/127.0.0.2/udp/12345".to_multiaddr().unwrap());

    // give it some time to connect
    thread::sleep(Duration::from_millis(500));

    // query
    let result = node2.query(&(b"key".to_vec()));
    assert!(result.is_some());
    assert_eq!(result.unwrap(), (b"value".to_vec()));

    // stop node1
    node1.stop();

    // query again
    let result = node2.query(&(b"key".to_vec()));
    assert!(result.is_some());
    assert_eq!(result.unwrap(), (b"value".to_vec()));

    // query for something non existent
    let result = node2.query(&(b"doesnotexist".to_vec()));
    assert!(result.is_none());

    // stop node2
    node2.stop();
}


fn new_udp_sha1_node(addr: &str) -> DHTService<sha1::SHA1Hasher> {
    // initialize DHT
    let hasher = sha1::SHA1Hasher::new();
    let mut service = DHTService::new(
        hasher,
        20,
        3,
        Duration::from_secs(15 * 60), // peer timeout
        Duration::from_secs(3), // communication timeout
        Duration::from_secs(60 * 60), // storage timeout
    );
    let udp_gateway = UdpGateway::new(addr);

    // register gateways
    service.register_gateway(udp_gateway);

    service
}

struct BadTwoByteHasher;
impl DHTHasher for BadTwoByteHasher {
    fn hash(data: &[u8]) -> Vec<u8> {
        if data.len() == 0 {
            return vec![0, 0];
        } else if data.len() == 1 {
            return vec![data[0], 0];
        } else {
            return data[..2].to_vec();
        }
    }

    fn get_hash_bytes_count() -> usize {
        2
    }
}

/// With this configuration, the routing table has 8 buckets with 3 nodes in each one of them
/// Id collisions are also very likely
fn new_udp_badhash_node(addr: &str) -> DHTService<BadTwoByteHasher> {
    // initialize DHT
    let hasher = BadTwoByteHasher {};
    let mut service = DHTService::new(
        hasher,
        3,
        1,
        Duration::from_secs(15 * 60), // peer timeout
        Duration::from_secs(3), // communication timeout
        Duration::from_secs(60 * 60), // storage timeout
    );
    let udp_gateway = UdpGateway::new(addr);

    // register gateways
    service.register_gateway(udp_gateway);

    service
}

#[test]
fn tiny_full_network() {
    // init logger
    init_logger();

    let n = 32;

    // init nodes
    let mut nodes: Vec<DHTService<BadTwoByteHasher>> = (1..n + 1)
        .map(|x| new_udp_badhash_node(&format!("127.0.1.{}:12345", x)))
        .collect();

    // start all nodes
    for (i, mut node) in nodes.iter_mut().enumerate() {
        node.start();
        let _ = node.connect(
            format!("/ip4/127.0.1.{}/udp/12345", i)
                .to_multiaddr()
                .unwrap(),
        );
        thread::sleep(Duration::from_secs(1)); //time to connect
    }

    // give them some time to connect
    //thread::sleep(Duration::from_secs(60));

    // save some value
    let _ = nodes[0].save(&(b"key".to_vec()), (b"value".to_vec()));

    // give it time to distribute
    thread::sleep(Duration::from_millis(1000));

    // query
    for i in 1..n {
        let result = nodes[i].query(&(b"key".to_vec()));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), (b"value".to_vec()));
    }

    //stop
    for mut node in nodes.iter_mut() {
        node.stop();
    }
}
