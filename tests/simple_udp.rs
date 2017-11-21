extern crate kademlia_dht;
extern crate multiaddr;
extern crate env_logger;

use kademlia_dht::DHTService;
use kademlia_dht::hash::sha1;
use kademlia_dht::gateway::UdpGateway;

use std::thread;
use std::time::Duration;
use multiaddr::Multiaddr;


#[test]
fn test_dht_start_and_stop() {
    // init logger
    env_logger::init().unwrap();

    // initialize DHT
    let hasher = sha1::SHA1Hasher::new();
    let mut service = DHTService::new(hasher, 20, 3, Duration::from_secs(15 * 60));
    let udp_gateway = UdpGateway::new("127.0.0.1:12345");
    let seed = Multiaddr::new("/ip4/127.0.0.1/udp/12346").unwrap();

    // register gateways
    service.register_gateway(udp_gateway);

    //start
    service.start(seed);

    //wait
    thread::sleep(Duration::from_secs(3));

    //stop
    service.stop();
}
