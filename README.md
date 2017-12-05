# Kademlia DHT in Rust

This is crate is supposed to make running DHT's easy.

## Design
The DHT node in this library runs always in separate thread. The public API is non-blocking
except for `query`. When you interact with the client, you are communicating with
the management thread - the thread handling all the logic. The management thread processes all
incoming messages, reacts to them and it handles your request. But actual network
communication is done separately.
The management thread runs alongside with gateway threads. Their only purpose is to handle
networking. There can be as many gateways as you wish and each one of them can communicate
in a different way, using different protocol and format. Build-in gateway is currently only
`UdpGateway`, which handles simple communication directly over UDP with straightforward
serialization format using `serde` crate. The only limitation in variaty of gateways is, that
the address must be representable using [multiaddr](https://github.com/multiformats/multiaddr).

This library provides best-effort access to DHT network. This is due to the nature of supported
network protocols - for example UDP. Queries can sometimes return nothing even though the value
exists somewhere in the network. This should happen only in cases, when the network connection
fails.

## Usage

To connect to DHT network, you need to instantiate a configured client, setup at least one
gateway, start the service. After that, you will need to know an address of at least one
other peer in network, so we can connect to it and seed our communication from him.

```rust
extern crate multiaddr;
extern crate kademlia_dht;

use std::time::Duration;
    
use kademlia_dht::DHTService;
use kademlia_dht::hash::sha1;
use kademlia_dht::gateway::UdpGateway;

use multiaddr::ToMultiaddr;

fn main() {
    let mut service = DHTService::new(
        sha1::SHA1Hasher::new(),        // configure hashing algoritm used
        20,                             // bucket size 
                                        // (#hash bits * bucketnumber = routing table max size)
        3,                              // query concurrency value
        Duration::from_secs(15 * 60),   // peer timeout (for how long after last connection 
                                        //               we consider node active)
        Duration::from_secs(3),         // communication timeout
        Duration::from_secs(60 * 60),   // storage (cache) timeout
    );
    
    // at least one gateway is required
    service.register_gateway(UdpGateway::new("127.0.0.13:12345"));
    
    // start service (spawns threads)
    service.start();
    
    // connect to seed specified by Multiaddr (can be more than one)
    service.connect("/ip4/127.0.0.17/udp/12345".to_multiaddr().unwrap());
    
    // now the client knows about the network and it takes some time to initialize at least 
    // the first connection
    
    // after a while, you can query
    let result = service.query(&vec![1u8, 8u8, 255u8]);
    
    // or save value to the network
    let result = service.save(&vec![1u8, 8u8, 255u8], vec![1u8, 8u8, 255u8, 254u8]);
    
    // and when you are done, you should stop the node, better to do it manually, but when dropped,
    // the DHTService will stop itself.
    service.stop();
}
```

You can have as many clients as you want in single application. 

## Detailed documentation

More information about this crate can be found in generated documentation by cargo or in source code;