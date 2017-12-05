# Example CLI DHT client

Simple SHA1 DHT client with command line interface.

## Compile and run

The program needs to know one IP and PORT on which it should listen.

```sh
# Compile and run
cargo run -- [ip] [port]

# Compile
cargo build --release
# run
target/release/example [ip] [port]
```

## CLI

Commands are accepted on stdin. Accepted commands are:

* **help**
    * show help
* **connect _\[multiaddr\]_**
    * connect to other peer, must be UDP multiaddr: `/ipX/[ip]/udp/[port]`
* **save _\[key\]_ _\[value\]_**
    * save some value under specified key to network
* **query _\[key\]_**
    * query value from DHT network
* **status**
    * display size of routing table

## Log

If you run the example with environment variable `RUST_LOG` set to `info`, you can see what the client is doing. Setting it to debug will cause it to print all the packets it handles.