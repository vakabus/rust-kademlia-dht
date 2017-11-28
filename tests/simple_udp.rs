extern crate kademlia_dht;
extern crate multiaddr;
extern crate env_logger;

use kademlia_dht::DHTService;
use kademlia_dht::hash::sha1;
use kademlia_dht::gateway::UdpGateway;

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

    let mut service = new_udp_node("127.0.0.1:12345");

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
    let mut node1 = new_udp_node("127.0.0.2:12345");
    let mut node2 = new_udp_node("127.0.0.3:12345");

    // start nodes
    node1.start();
    node2.start();

    node1.save(&(b"key".to_vec()), (b"value".to_vec()));
    node2.connect("/ip4/127.0.0.2/udp/12345".to_multiaddr().unwrap());

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


fn new_udp_node(addr: &str) -> DHTService<sha1::SHA1Hasher> {
    // initialize DHT
    let hasher = sha1::SHA1Hasher::new();
    let mut service = DHTService::new(
        hasher,
        20,
        3,
        Duration::from_secs(15 * 60), // peer timeout
        Duration::from_secs(3), // communication timeout
    );
    let udp_gateway = UdpGateway::new(addr);

    // register gateways
    service.register_gateway(udp_gateway);

    service
}
