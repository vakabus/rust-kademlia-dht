use std::collections::{HashMap, BTreeMap};
use std::sync::mpsc::{Sender, Receiver};
use std::thread::JoinHandle;
use std::time::{Instant, Duration};
use multiaddr::Multiaddr;

use id::*;
use msg::*;
use peer::*;
use routing::*;
use gateway::*;
#[macro_use]
use log::*;

pub struct DHTManagement {
    peer_id: UID,
    config: DHTConfig,
    data_store: HashMap<UID, Vec<u8>>,
    routing_table: RoutingTable,
    running_gateways: Vec<RunningGateway>,
    incoming_msg: Receiver<Msg>,
    control_channel: Receiver<DHTControlMsg>,
    response_channel: Sender<(UID, Option<Vec<u8>>)>,
}

pub struct DHTConfig {
    pub concurrency_degree: usize,
    pub hash_size: usize,
    pub peer_timeout: Duration,
    pub bucket_size: usize,
    pub msg_timeout: Duration,
}

pub struct RunningGateway {
    pub command_sender: Sender<ControlMsg>,
    pub handle: JoinHandle<()>,
    pub send_validator: Box<SendAbilityChecker + Send>,
}

struct PendingOperations {
    packets: HashMap<UID, (PendingOperationExec, Instant)>,
    peer_lookups: HashMap<UID, PendingLookup>,
}

enum PendingOperationExec {
    Pong {
        exec: fn(&mut DHTManagement, omsg: Option<Msg>, old: Peer, new: Peer),
        old: Peer,
        new: Peer,
    },
    PongSimple { exec: fn(&mut DHTManagement, omsg: Option<Msg>), },
    Lookup {
        exec: fn(&mut DHTManagement,
                 pending: &mut PendingOperations,
                 msg: Option<Msg>,
                 target: UID),
        target: UID,
    },
}

impl PendingOperationExec {
    fn exec(self, mgmt: &mut DHTManagement, pending: &mut PendingOperations, omsg: Option<Msg>) {
        match self {
            PendingOperationExec::Pong { exec, old, new } => exec(mgmt, omsg, old, new),
            PendingOperationExec::Lookup { exec, target } => exec(mgmt, pending, omsg, target),
            PendingOperationExec::PongSimple { exec } => exec(mgmt, omsg),
        }
    }
}

enum ResultExec {
    Peer { exec: fn(&mut DHTManagement, UID, Vec<Peer>), },
    Value { exec: fn(&mut DHTManagement, UID, Option<Vec<u8>>), },
}

impl PendingOperations {
    fn new() -> PendingOperations {
        PendingOperations {
            packets: HashMap::new(),
            peer_lookups: HashMap::new(),
        }
    }
}

struct PendingLookup {
    peers: Vec<(Peer, bool)>,
    result: ResultExec,
}

pub enum DHTControlMsg {
    Stop,
    Save { key: UID, value: Vec<u8> },
    Query { key: UID },
    Connect { addr: Multiaddr },
}

impl DHTManagement {
    pub fn new(
        hash_size: usize,
        bucket_size: usize,
        concurrency_degree: usize,
        peer_timeout: Duration,
        msg_timeout: Duration,
        incoming_msg: Receiver<Msg>,
        control_channel: Receiver<DHTControlMsg>,
        response_channel: Sender<(UID, Option<Vec<u8>>)>,
    ) -> DHTManagement {
        let my_peer_id = UID::random(hash_size);
        DHTManagement {
            config: DHTConfig {
                hash_size: hash_size,
                bucket_size: bucket_size,
                concurrency_degree: concurrency_degree,
                peer_timeout: peer_timeout,
                msg_timeout: msg_timeout,
            },
            running_gateways: Vec::new(),
            peer_id: my_peer_id.clone(),
            data_store: HashMap::new(),
            routing_table: RoutingTable::new(my_peer_id, bucket_size, peer_timeout),
            incoming_msg,
            control_channel,
            response_channel,
        }
    }


    fn _send_msg(&self, msg: Msg) -> bool {
        debug!("SEND Msg: {:?}", msg);

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

    fn msg_ping(
        &mut self,
        pending: &mut PendingOperations,
        addr: &Multiaddr,
        f: PendingOperationExec,
    ) {
        let msg = Msg::new_ping(&self.peer_id, addr);
        pending.packets.insert(
            msg.msg_id.clone(),
            (f, Instant::now()),
        );
        self._send_msg(msg);
    }

    fn msg_pong(&self, msg_id: UID, dst: &Multiaddr) {
        self._send_msg(Msg::new_pong(&self.peer_id, msg_id, dst));
    }

    fn msg_find_node(
        &mut self,
        pending: &mut PendingOperations,
        id: &UID,
        peer: &Peer,
        f: PendingOperationExec,
    ) {
        let msg = Msg::new_find_node(&self.peer_id, &peer.addr, &id);
        pending.packets.insert(
            msg.msg_id.clone(),
            (f, Instant::now()),
        );
        self._send_msg(msg);
    }

    fn msg_find_value(
        &mut self,
        pending: &mut PendingOperations,
        dst: &Multiaddr,
        key: &UID,
        f: PendingOperationExec,
    ) {
        let msg = Msg::new_find_value(&self.peer_id, dst, key);
        pending.packets.insert(
            msg.msg_id.clone(),
            (f, Instant::now()),
        );
        self._send_msg(msg);
    }

    fn msg_value_found(&self, msg_id: UID, dst: &Multiaddr, key: &UID) {
        let value: &Vec<u8> = self.data_store.get(key).unwrap();
        self._send_msg(Msg::new_value_found(
            &self.peer_id,
            msg_id,
            &dst,
            key.clone(),
            value,
        ));
    }

    fn msg_list_peers(&self, msg_id: UID, dst: &Multiaddr, peer_id: &UID) {
        let peers = self.routing_table.get_k_nearest_peers(
            &peer_id,
            self.config.bucket_size,
        );
        let peers = peers.into_iter().map(|x| MsgPeer::from(x)).collect();
        self._send_msg(Msg::new_list_peers(&self.peer_id, msg_id, dst, peers));
    }

    fn msg_store(&self, dst: &Multiaddr, key: &UID) {
        let value: &Vec<u8> = self.data_store.get(key).unwrap();
        let msg = Msg::new_store(&self.peer_id, dst, key.clone(), value);
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

    fn routing_table_insert(
        &mut self,
        pending: &mut PendingOperations,
        peer_id: Option<UID>,
        addr: Multiaddr,
    ) {
        if let Some(peer_id) = peer_id {
            let res = self.routing_table.update_or_insert_peer(peer_id, addr);
            if let Err((new, old)) = res {
                let old = old.clone();
                self.msg_ping(
                    pending,
                    &old.addr.clone(),
                    PendingOperationExec::Pong {
                        exec: DHTManagement::handle_incoming_pong,
                        old: old,
                        new: new,
                    },
                );
            }
        } else {
            self.msg_ping(
                pending,
                &addr,
                PendingOperationExec::PongSimple {
                    exec: DHTManagement::handle_incoming_pong_simple,
                },
            )
        }
    }

    fn handle_incoming_pong(&mut self, msg: Option<Msg>, old: Peer, new: Peer) {
        if let Some(msg) = msg {
            // the long known peer responded, lets drop the new one
            self.routing_table.update_peer(&old.peer_id);
        } else {
            self.routing_table.remove_peer(&old.peer_id);
            let _ = self.routing_table.insert_peer(new);
        };
    }

    fn handle_incoming_pong_simple(&mut self, msg: Option<Msg>) {
        if let Some(msg) = msg {
            let has_peer = self.routing_table.has_peer(&msg.peer_id);
            if has_peer {
                // the peer already exist, do nothing
            } else {
                self.routing_table.insert_peer(
                    Peer::new(msg.peer_id, msg.addr),
                );
            }
        }
    }

    fn locale_k_closest_nodes(
        &mut self,
        pending: &mut PendingOperations,
        peer_id: UID,
        f: fn(&mut DHTManagement, UID, Vec<Peer>) -> (),
    ) {
        // get alpha nearest known nodes
        let peers = self.routing_table.get_k_nearest_peers(
            &peer_id,
            self.config.concurrency_degree,
        );

        if peers.len() == 0 {
            f(self, peer_id, vec![]);
            return;
        }

        // save work data for future use
        let data = PendingLookup {
            peers: peers.clone().into_iter().map(|x| (x, true)).collect(),
            result: ResultExec::Peer { exec: f },
        };
        pending.peer_lookups.insert(peer_id.clone(), data);

        // send alpha lookup requests
        for peer in peers {
            self.msg_find_node(
                pending,
                &peer_id,
                &peer,
                PendingOperationExec::Lookup {
                    exec: DHTManagement::handle_incoming_lookup,
                    target: peer_id.clone(),
                },
            );
        }
    }

    fn find_value_for_key(
        &mut self,
        pending: &mut PendingOperations,
        key: UID,
        f: fn(&mut DHTManagement, UID, Option<Vec<u8>>),
    ) {
        // get alpha nearest known nodes
        let peers = self.routing_table.get_k_nearest_peers(
            &key,
            self.config.concurrency_degree,
        );

        // save work data for future use
        let data = PendingLookup {
            peers: peers.clone().into_iter().map(|x| (x, true)).collect(),
            result: ResultExec::Value { exec: f },
        };
        pending.peer_lookups.insert(key.clone(), data);

        // send alpha lookup requests
        for peer in peers {
            self.msg_find_value(
                pending,
                &peer.addr,
                &key,
                PendingOperationExec::Lookup {
                    exec: DHTManagement::handle_incoming_lookup,
                    target: key.clone(),
                },
            );
        }
    }

    fn handle_incoming_lookup(
        &mut self,
        pending: &mut PendingOperations,
        msg: Option<Msg>,
        target: UID,
    ) {
        // get lookup data, terminate if there are none
        let data = pending.peer_lookups.remove(&target);
        if data.is_none() {
            return;
        };
        let mut data = data.unwrap();

        // check new msg
        if let Some(msg) = msg {
            match msg.msg_type {
                MsgType::RES_LIST_PEERS { peers } => {
                    // merge
                    data.peers.append(&mut peers
                        .into_iter()
                        .map(|x| x.to_peer())
                        .filter(|x| x.is_some())
                        .map(|x| (x.unwrap(), false))
                        .collect());
                    // sort (nearest peers end up first)
                    data.peers.sort_unstable_by(|a, b| {
                        a.0.peer_id.distance(&target).cmp(
                            &(b.0.peer_id.distance(
                                &target,
                            )),
                        )
                    });
                }
                MsgType::RES_VALUE_FOUND { key, value } => {
                    // check, if we are looking for value or peers
                    if let ResultExec::Value { exec } = data.result {
                        // deliver result
                        if (key == target) {
                            exec(self, key, Some(value));
                            return;
                        } else {
                            eprintln!("Unexpected key value pair. Requested a different one");
                        }
                    } else {
                        eprintln!("Unexpected message. Value arrived when requesting peers only.");
                    }
                }
                _ => {
                    eprintln!("Unexpected message.");
                }
            }
        }

        // pick nearest unprocessed peer closer than bucket size + concurrency size from top
        // of the list
        let next = data.peers
            .iter()
            .enumerate()
            .map(|x| (x.0, x.1.clone()))
            .filter(|x| !(x.1).1)
            .next();

        // get index of next peer
        // if no next peer, the index is made big enough to be considered out of range
        let index = next.as_ref().map(|x| x.0).unwrap_or(
            self.config.concurrency_degree +
                self.config.bucket_size,
        );

        //if found peer is not in the first alpha+bucket_size, consider the lookup as finished
        if index >= self.config.concurrency_degree + self.config.bucket_size {
            // we should be finished here

            // deliver result
            let bucket_size = self.config.bucket_size;
            match data.result {
                ResultExec::Peer { exec } => {
                    exec(
                        self,
                        target,
                        data.peers
                            .into_iter()
                            .enumerate()
                            .filter(|x| x.0 < bucket_size)
                            .map(|x| (x.1).0)
                            .collect(),
                    );
                }
                ResultExec::Value { exec } => {
                    exec(self, target, None);
                }
            }

            // nothing to do, terminate (the other requests will terminate too, because they
            // no longer have access to neccessary data)
            return;
        }

        let (_, ppp) = next.unwrap().clone();
        let (peer, _) = ppp;

        // mark it as being processed
        data.peers = data.peers
            .into_iter()
            .map(|x| if x.0.peer_id == peer.peer_id {
                (x.0, true)
            } else {
                x
            })
            .collect();
        //send lookup request
        self.msg_find_node(
            pending,
            &target,
            &peer,
            PendingOperationExec::Lookup {
                exec: DHTManagement::handle_incoming_lookup,
                target: target.clone(),
            },
        );
        //save data for future use
        pending.peer_lookups.insert(target, data);
    }


    fn handle_incoming_network_messages(&mut self, pending: &mut PendingOperations) {
        // handle received
        let received = self.incoming_msg.try_recv();
        if let Ok(msg) = received {
            debug!("RECV Msg: {:?}", msg);
            match msg.msg_type {
                // PING request
                MsgType::REQ_PING => {
                    self.msg_pong(msg.msg_id, &msg.addr);
                    self.routing_table_insert(pending, Some(msg.peer_id), msg.addr);
                }
                MsgType::REQ_FIND_NODE { peer_id } => {
                    // reply with list of best known peers
                    self.msg_list_peers(msg.msg_id, &msg.addr, &peer_id);
                    self.routing_table_insert(pending, Some(msg.peer_id), msg.addr);
                }
                MsgType::REQ_FIND_VALUE { key } => {
                    if self.data_store.contains_key(&key) {
                        // send the value
                        self.msg_value_found(msg.msg_id, &msg.addr, &key);
                    } else {
                        // send closest known peers
                        self.msg_list_peers(msg.msg_id, &msg.addr, &UID::from(key));
                    }
                    self.routing_table_insert(pending, Some(msg.peer_id), msg.addr);
                }
                MsgType::REQ_STORE { key, value } => {
                    self.data_store.insert(key, value);
                    self.routing_table_insert(pending, Some(msg.peer_id), msg.addr);
                }
                // any response
                _ => {
                    self.routing_table_insert(pending, Some(msg.peer_id.clone()), msg.addr.clone());
                    let pend = pending.packets.remove(&msg.msg_id);
                    if let Some((run, _)) = pend {
                        run.exec(self, pending, Some(msg));
                    }
                }
            }
        }

        // handle waiting packets
        let to_remove: Vec<UID> = pending
            .packets
            .iter()
            .filter(|x| ((x.1).1).elapsed() > self.config.msg_timeout)
            .map(|x| (x.0).clone())
            .collect();
        for id in to_remove {
            let (run, _) = pending.packets.remove(&id).unwrap();
            run.exec(self, pending, None);
        }
    }
}

pub fn run(mut node: DHTManagement) -> DHTManagement {
    let mut pending = PendingOperations::new();

    loop {
        // process incoming message
        node.handle_incoming_network_messages(&mut pending);

        // Process incoming commands from main application thread
        {
            let maybe_command = node.control_channel.try_recv();
            if let Ok(cmd) = maybe_command {
                // process incoming command
                match cmd {
                    DHTControlMsg::Stop => return node,
                    DHTControlMsg::Save { key, value } => {
                        // save the data locally
                        node.data_store.insert(key.clone(), value.clone());
                        // distribute the data further into the network
                        node.locale_k_closest_nodes(&mut pending, key, result_store);
                    }
                    DHTControlMsg::Query { key } => {
                        // check if we already have it
                        if node.data_store.contains_key(&key) {
                            let value = node.data_store.get(&key).unwrap().clone();
                            result_query(&mut node, key, Some(value));
                        } else {
                            node.find_value_for_key(&mut pending, key, result_query)
                        }
                    }
                    DHTControlMsg::Connect { addr } => {
                        node.routing_table_insert(&mut pending, None, addr);
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
        // TODO hardcoded constant
        if node.routing_table.len() < node.config.bucket_size * 2 {
            // search for myself
            let mid = node.peer_id.clone();
            node.locale_k_closest_nodes(&mut pending, mid, result_nop);
        }

        // TODO ping old peers?
        // ping old peers
        /*{
            let old = node.routing_table.remove_old_peers();
            for peer in old {
                node.msg_ping(&peer);
            }
        }*/
    }
}

fn result_nop(_: &mut DHTManagement, _: UID, _: Vec<Peer>) {}

fn result_store(mgmt: &mut DHTManagement, peer_id: UID, peers: Vec<Peer>) {
    for peer in peers {
        mgmt.msg_store(&peer.addr, &peer_id);
    }
}

fn result_query(mgmt: &mut DHTManagement, key: UID, value: Option<Vec<u8>>) {
    // save value for future use
    if value.is_some() {
        mgmt.data_store.insert(key.clone(), value.clone().unwrap());
    }
    mgmt.response_channel.send((key, value));
}
