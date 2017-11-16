use std::collections::{HashMap, BTreeMap};
use std::sync::mpsc::{Sender, Receiver};
use std::thread::JoinHandle;
use std::time::{Instant, Duration};
use multiaddr::Multiaddr;

use msg::*;
use peer::*;
use routing::*;
use gateway::*;

pub struct DHTManagement {
    peer_id: PeerID,
    config: DHTConfig,
    data_store: HashMap<Vec<u8>, Vec<u8>>,
    routing_table: RoutingTable,
    running_gateways: Vec<RunningGateway>,
    pending: PendingOperations,
}

pub struct DHTConfig {
    pub concurrency_degree: usize,
    pub hash_size: usize,
    pub peer_timeout: Duration,
    pub bucket_size: usize,
}

pub struct RunningGateway {
    pub command_sender: Sender<ControlMsg>,
    pub handle: JoinHandle<()>,
    pub send_validator: Box<SendAbilityChecker + Send>,
}

struct PendingOperations {
    packets: HashMap<MsgID, (Box<FnOnce(&mut DHTManagement, Option<Msg>) -> ()>, Instant)>,
    peer_lookups: HashMap<PeerID, PendingNodeLookup>,
}

struct PendingNodeLookup {
    peers: Vec<(Peer, bool)>,
    result: Box<FnOnce(Vec<Peer>) -> ()>,
}

impl PendingOperations {
    fn new() -> PendingOperations {
        PendingOperations {
            packets: HashMap::new(),
            peer_lookups: HashMap::new(),
        }
    }
}

pub enum DHTControlMsg {
    Stop,
    Save { key: Vec<u8>, value: Vec<u8> },
    Query {key: Vec<u8>},
}

impl DHTManagement {
    pub fn new(
        hash_size: usize,
        bucket_size: usize,
        concurrency_degree: usize,
        peer_timeout: Duration,
    ) -> DHTManagement {
        let my_peer_id = PeerID::random(hash_size);
        DHTManagement {
            config: DHTConfig{
                hash_size: hash_size,
                bucket_size: bucket_size,
                concurrency_degree: concurrency_degree,
                peer_timeout: peer_timeout,
            },
            running_gateways: Vec::new(),
            peer_id: my_peer_id.clone(),
            data_store: HashMap::new(),
            routing_table: RoutingTable::new(my_peer_id, bucket_size, peer_timeout),
            pending: PendingOperations::new(),
        }
    }


    fn _send_msg(&self, msg: Msg) -> bool {
        //TODO implement round robin
        for (i, rgw) in self.running_gateways.iter().enumerate() {
            if rgw.send_validator.can_send(&msg.addr) {
                // send it
                let s = rgw.command_sender.send(ControlMsg::SendMsg(msg));
                return s.is_ok();
            }
        }
        eprintln!("Can't send {:?}", msg);
        false
    }

    fn msg_ping(&mut self, peer: &Peer, f: Box<FnOnce(&mut DHTManagement, Option<Msg>) -> ()>) {
        let msg = Msg::new_ping(&self.peer_id, &peer);
        self.pending.packets.insert(msg.id.clone(), (f, Instant::now()));
        self._send_msg(msg);
    }

    fn msg_pong(&self, msg_id: MsgID, dst: &Multiaddr) {
        self._send_msg(Msg::new_pong(&self.peer_id, msg_id, dst));
    }

    fn msg_find_node(&mut self, id: &PeerID, peer: &Peer, f: Box<FnOnce(&mut DHTManagement, Option<Msg>) -> ()>) {
        let msg = Msg::new_find_node(&self.peer_id, &peer.addr, &id);
        self.pending.packets.insert(msg.id.clone(), (f, Instant::now()));
        self._send_msg(msg);
    }

    fn msg_find_value(&mut self, dst: &Multiaddr, key: &Vec<u8>, f: Box<FnOnce(&mut DHTManagement, Option<Msg>) -> ()>) {
        let msg = Msg::new_find_value(&self.peer_id, dst, key);
        self.pending.packets.insert(msg.id.clone(), (f, Instant::now()));
        self._send_msg(msg);
    }

    fn msg_value_found(&self, msg_id: MsgID, dst: &Multiaddr, key: &Vec<u8>) {
        let value: &Vec<u8> = self.data_store.get(key).unwrap();
        self._send_msg(Msg::new_value_found(&self.peer_id, msg_id,&dst,key,value,));
    }

    fn msg_list_peers(&self, msg_id: MsgID, dst: &Multiaddr, peer_id: &PeerID) {
        let peers = self.routing_table.get_k_nearest_peers(&peer_id, self.config.bucket_size);
        let peers = peers.into_iter().map(|x| MsgPeer::from(x)).collect();
        self._send_msg(Msg::new_list_peers(&self.peer_id, msg_id, dst, peers));
    }

    fn msg_store(&self, dst: &Multiaddr, key: &Vec<u8>) {
        let value: &Vec<u8> = self.data_store.get(key).unwrap();
        let msg = Msg::new_store(&self.peer_id, dst, key, value);
        self._send_msg(msg);
    }

    pub fn add_running_gateway(
        &mut self,
        control_send: Sender<ControlMsg>,
        handle: JoinHandle<()>,
        validator: Box<SendAbilityChecker + Send>,
    ) {
        self.running_gateways.push(RunningGateway {
            command_sender: control_send,
            handle: handle,
            send_validator: validator,
        });
    }

    pub fn drain_running_gateways(&mut self) -> ::std::vec::Drain<RunningGateway> {
        self.running_gateways.drain(..)
    }

    fn routing_table_insert(&mut self, peer_id: PeerID, addr :Multiaddr) {
        let res = self.routing_table.update_or_insert_peer(peer_id, addr);
        if let Err((new, old)) = res {
            self.msg_ping(&old, Box::from(move |mgmt: &mut DHTManagement, msg: Option<Msg>| {
                if let Some(msg) = msg {
                    // the long known peer responded, lets drop the new one
                    mgmt.routing_table.update_peer(&old.peer_id);
                } else {
                    mgmt.routing_table.remove_peer(&old.peer_id);
                    mgmt.routing_table.insert_peer(new);
                };
            }));
        }
    }

    fn locale_k_closest_nodes(&mut self, peer_id: PeerID, f: Box<FnOnce(Vec<Peer>)-> ()>) {
        // get alpha nearest known nodes
        let peers = self.routing_table.get_k_nearest_peers(&peer_id, self.config.concurrency_degree);

        // save work data for future use
        let data = PendingNodeLookup{
            peers: peers.into_iter().map(|x| (x, true)).collect(),
            result: f,
        };
        self.pending.peer_lookups.insert(peer_id, data);

        // send alpha lookup requests
        for (peer, _) in data.peers {
            self._spawn_lookup_request(peer.clone(), peer_id.clone());
        }
    }

    fn _spawn_lookup_request(&mut self, peer: Peer, target: PeerID) -> () {
        let process_incoming = move |mgmt: &mut DHTManagement, msg: Option<Msg>| {
            // get lookup data, terminate if there are none
            let data = mgmt.pending.peer_lookups.remove(&target);
            if data.is_none() {return;};
            let data = data.unwrap();

            // check new msg
            if let Some(msg) = msg {
                if let MsgType::RES_LIST_PEERS{ peers } = msg.msg_type {
                    // merge
                    data.peers.append(&mut peers.into_iter()
                                        .map(|x| x.to_peer())
                                        .filter(|x| x.is_some())
                                        .map(|x| (x.unwrap(), false))
                                        .collect());
                    // sort (nearest peers end up first)
                    data.peers.sort_unstable_by(|a, b| {
                        a.0.peer_id.distance(&target).cmp(
                            &(b.0.peer_id.distance(&target)),
                        )
                    });
                }
            }

            // pick nearest unprocessed peer closer than bucket size + concurrency size from top
            // of the list
            let next = data.peers.iter().enumerate().filter(|x| {!(x.1).1}).next();
            if next.is_none() {
                // no unprocessed peer found, terminate
                return;
            }

            let (index, ppp) = next.unwrap();
            let (peer, _) = *ppp;

            //if found peer is not in the first alpha+bucket_size, consider the lookup as finished
            if index >= mgmt.config.concurrency_degree + mgmt.config.bucket_size {
                // we should be finished here

                // deliver result
                (data.result)(data.peers.into_iter()
                        .enumerate()
                        .filter(|x| x.0 < mgmt.config.bucket_size)
                        .map(|x| (x.1).0)
                        .collect()
                    );
                // nothing to do, terminate (the other requests will terminate too, because they
                // no longer have access to neccessary data)
                return;
            }

            // mark it as being processed
            data.peers = data.peers.into_iter().map(|x| {
                if x.0.peer_id == peer.peer_id {
                    (x.0, true)
                } else {
                    x
                }
            }).collect();
            //send lookup request
            mgmt._spawn_lookup_request(peer.clone(), target.clone());
            //save data for future use
            mgmt.pending.peer_lookups.insert(target, data);
        };

        self.msg_find_node(&target, &peer, Box::from(process_incoming));
    }

    fn handle_incoming_network_messages(&mut self, incoming_msg: &Receiver<Msg>) {
        let received = incoming_msg.try_recv();
        if let Ok(msg) = received {
            match msg.msg_type {
                // PING request
                MsgType::REQ_PING => {
                    self.msg_pong(msg.id, &msg.addr);
                    self.routing_table_insert(msg.peer_id, msg.addr);
                },
                MsgType::REQ_FIND_NODE { peer_id } => {
                    // reply with list of best known peers
                    self.msg_list_peers(msg.id, &msg.addr, &peer_id);
                    self.routing_table_insert(msg.peer_id, msg.addr);
                },
                MsgType::REQ_FIND_VALUE { key } => {
                    if self.data_store.contains_key(&key) {
                        // send the value
                        self.msg_value_found(msg.id, &msg.addr, &key);
                    } else {
                        // send closest known peers
                        self.msg_list_peers(msg.id, &msg.addr, &PeerID::from(key));
                    }
                    self.routing_table_insert(msg.peer_id, msg.addr);
                },
                MsgType::REQ_STORE { key, value } => {
                    unimplemented!();
                },
                // any response
                _ => {
                    let pend = self.pending.packets.remove(&msg.id);
                    if let Some((run, duration)) = pend {
                        run(self, Some(msg));
                    }
                }
            }
        }
    }
}

pub fn run(
    incoming_msg: Receiver<Msg>,
    control_channel: Receiver<DHTControlMsg>,
    mut node: DHTManagement,
) -> DHTManagement {
    loop {
        // process incoming message
        node.handle_incoming_network_messages(&incoming_msg);

        // Process incoming commands from main application thread
        {
            let maybe_command = control_channel.try_recv();
            if let Ok(cmd) = maybe_command {
                // process incoming command
                match cmd {
                    DHTControlMsg::Stop => return node,
                    DHTControlMsg::Save { key, value } => {
                        // save the data locally
                        node.data_store.insert(key.clone(), value.clone());
                        // distribute the data further into the network
                        node.locale_k_closest_nodes(
                            PeerID::from(key.clone()),
                            Box::from(move |x: Vec<Peer>| {
                                for peer in x {
                                    node.msg_store(&peer.addr, &key);
                                }
                            }),
                        );
                    },
                    DHTControlMsg::Query {key} => {
                        // TODO find value
                    }
                }
            }
        }

        // Handle gateways diing
        {
            // TODO implement
            // it might be caused by disconnecting from the network, or something similar
            // the best would be to report it to the main app thread and remove it
        }

        // handle not enough peers in routing table
        {
            let not_full = node.routing_table.list_not_full_buckets();
            for bucket_id in not_full {
                // generate random id from within the bucket
                let mut rand_peer_id = PeerID::random_within_bucket(node.peer_id.len(), bucket_id);
                // find nearest known peers
                let peers: Vec<Peer> = {
                    let peers = node.routing_table.get_k_nearest_peers(&rand_peer_id);
                    peers.into_iter().map(|x| x.clone()).collect()
                };
                // send lookup to alpha peers
                let n = if node.config.concurrency_degree > peers.len() {peers.len()} else {node.config.concurrency_degree};
                for peer in peers[..n].iter() {
                    node.msg_find_node(&rand_peer_id, &peer);
                }
            }
        }

        // TODO operation too expensive, do not perform it every time
        // ping old peers
        {
            let old = node.routing_table.remove_old_peers();
            for peer in old {
                node.msg_ping(&peer);
            }
        }
    }
}
