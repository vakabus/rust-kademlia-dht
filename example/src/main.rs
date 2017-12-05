extern crate kademlia_dht;
extern crate log;
extern crate multiaddr;
extern crate env_logger;
extern crate argparse;

use argparse::*;
use multiaddr::*;
use kademlia_dht::*;
use kademlia_dht::gateway::*;
use kademlia_dht::hash::*;

use std::time::Duration;
use std::io;
use std::io::prelude::*;

fn args() -> (String, u16) {
    let mut ip: String = "0.0.0.0".to_owned();
    let mut port: u16 = 12345;

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Greet somebody.");
        ap.refer(&mut ip)
            .add_argument("ip", Store, "IP address to listen on");
        ap.refer(&mut port)
            .add_argument("port", Store,"Port to listen on");
        ap.parse_args_or_exit();
    }

    (ip, port)
}

fn flush() {
    io::stdout().flush().unwrap();
}



fn main() {
    // parse arguments
    let (ip, port) = args();

    // init logger
    env_logger::init().unwrap();

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

    let udp_gateway = UdpGateway::new(format!("{}:{}", ip, port).as_str());

    // register gateways
    service.register_gateway(udp_gateway);

    // start DHT
    service.start();

    println!("Press ^D to exit...");
    print!("> ");
    flush();

    // read loop
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.expect("stdio error");

        if line.len() != 0 {
            if line == "help" {
                println!("Allowed commands:");
                println!("  help                      - display help");
                println!("  connect [multiaddr str]   - try to connect to other peer");
                println!("  save [key] [value]        - save data in to the network");
                println!("  query [key]               - query the network for some data");
                println!("  status                    - get routing table size");
                println!("");
                println!("Arguments can't contain spaces", );
                println!("To exit press ^D");

            } else if line.starts_with("connect") {
                let sp: Vec<&str> = line.split(" ").collect();
                if sp.len() != 2 {
                    println!("Invalid number of arguments");
                } else {
                    let ma = sp[1].to_multiaddr();
                    if let Ok(ma) = ma {
                        let res = service.connect(ma);
                        if res.is_ok() {
                            println!("Connection initiated...");
                        } else {
                            println!("Failed to send command to DHT management thread.");
                        }
                    } else {
                        println!("Can't parse multiaddr");
                    }
                }
            } else if line.starts_with("save") {
                let sp: Vec<&str> = line.split(" ").collect();
                if sp.len() != 3 {
                    println!("Invalid number of arguments");
                } else {
                    let res = service.save(&sp[1].to_owned().into_bytes(), sp[2].to_owned().into_bytes());
                    if res.is_ok() {
                        println!("Data saving initiated...");
                    } else {
                        println!("Failed to send command to DHT management thread.");
                    }
                }
            } else if line.starts_with("query") {
                let sp: Vec<&str> = line.split(" ").collect();
                if sp.len() != 2 {
                    println!("Invalid number of arguments");
                } else {
                    println!("Initiating query...");
                    let res = service.query(&sp[1].to_owned().into_bytes());
                    println!("Result: {}", res.map(|v| String::from_utf8(v).unwrap_or("<can't decode>".to_owned())).unwrap_or("<no data>".to_owned()));
                }
            } else if line.starts_with("status") {
                println!("routing table size: {}", service.get_number_of_known_peers());
            } else {
                println!("Unknown command. Try `help`...");
            }
        }

        print!("> ");
        flush();
    }


    println!("\n\nStopping DHT example...");

    // stop dht
    service.stop();
}
