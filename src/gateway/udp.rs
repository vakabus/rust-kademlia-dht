use gateway::{MsgGateway, SendAbilityChecker, ProtocolBasedSendAbilityChecker};
use msg::Msg;
use multiaddr::Multiaddr;
use std::net::{SocketAddr, UdpSocket, IpAddr};
use std::time::Duration;
use std::str::FromStr;
use gateway::serialize::binary::{serialize, parse, BinMsg};

/// UDP gateway implementation using `serde` crate for serialization.
pub struct UdpGateway {
    socket: UdpSocket,
}

impl UdpGateway {
    pub fn new(addr: &str) -> Box<UdpGateway> {
        let socket = UdpSocket::bind(addr).expect("Failed to bind UDP socket.");
        let _ = socket.set_read_timeout(Some(Duration::from_millis(20)));
        let gw = UdpGateway { socket };

        Box::from(gw)
    }

    fn _recv(&self) -> Option<BinMsg> {
        // the huge buffer can't be removed, because if the datagram does not fit,
        // the rest of it would be discarded
        let mut buff = [0u8; 65536];
        match self.socket.recv_from(&mut buff) {
            Ok((len, addr)) => {
                let maddr = convert_address(addr);
                let msg = BinMsg::new(Vec::from(&buff[..len]), maddr);
                Some(msg)
            }
            Err(_) => None,
        }
    }

    fn _send(&self, msg: BinMsg) -> bool {
        let res = self.socket.send_to(
            msg.get_payload(),
            extract_address(msg.get_addr().clone()),
        );

        res.is_ok()
    }

    fn get_address(&self) -> Multiaddr {
        convert_address(self.socket.local_addr().expect(
            "UDPGateway: Did not bind to any address.",
        ))
    }
}

impl MsgGateway for UdpGateway {
    fn get_send_ability_checker(&self) -> Box<SendAbilityChecker + Send> {
        Box::from(ProtocolBasedSendAbilityChecker::new(self.get_address()))
    }

    fn recv(&mut self) -> Option<Msg> {
        let msg = self._recv();
        match msg {
            Some(msg) => {
                match parse(msg) {
                    Ok(msg) => Some(msg),
                    Err(_) => None,
                }
            }
            None => None,
        }
    }

    fn send(&mut self, msg: Msg) -> bool {
        let msg = serialize(msg);
        self._send(msg)
    }
}

fn convert_address(addr: SocketAddr) -> Multiaddr {
    match addr {
        SocketAddr::V4(a) => Multiaddr::new(&format!("/ip4/{}/udp/{}", a.ip(), a.port())),
        SocketAddr::V6(a) => Multiaddr::new(&format!("/ip6/{}/udp/{}", a.ip(), a.port())),
    }.expect("UDPGateway: Error parsing address.")
}

fn extract_address(ma: Multiaddr) -> SocketAddr {
    //TODO sure there must be better way!
    let a: Vec<String> = ma.to_string()
        .split('/')
        .map(|x| x.to_string().clone())
        .collect();
    SocketAddr::new(
        IpAddr::from_str(&a[2]).unwrap(),
        a[4].parse::<u16>().unwrap(),
    )
}


#[cfg(test)]
mod tests {

    use std::thread;
    use gateway::udp::UdpGateway;
    use multiaddr::ToMultiaddr;
    use gateway::serialize::binary::*;

    #[test]
    fn test_udp_send_recv_internal() {
        let p = vec![1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8];

        let gw1 = UdpGateway::new("127.0.0.3:12345");
        let gw2 = UdpGateway::new("127.0.0.2:12345");

        thread::spawn(move || {
            let _ = gw1._send(BinMsg::new(
                p.clone(),
                "/ip4/127.0.0.2/udp/12345".to_multiaddr().unwrap(),
            ));
        });

        let msg = gw2._recv().unwrap();
        assert_eq!(msg.get_addr().to_string(), "/ip4/127.0.0.3/udp/12345");
        assert_eq!(
            msg.get_payload(),
            &vec![1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8]
        );
    }
}
