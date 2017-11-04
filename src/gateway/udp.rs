use gateway::{MsgGateway, SendAbilityChecker, ProtocolBasedSendAbilityChecker};
use msg::{BinMsg, Msg};
use multiaddr::Multiaddr;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

pub struct UdpGateway {
    socket: UdpSocket,
}

impl UdpGateway {
    pub fn new(addr: &str) -> Box<UdpGateway> {
        let mut socket = UdpSocket::bind(addr).expect("Failed to bind UDP socket.");
        socket.set_read_timeout(Some(Duration::from_millis(20)));
        let gw = UdpGateway { socket };

        Box::from(gw)
    }
}

impl MsgGateway for UdpGateway {
    fn get_send_ability_checker(&self) -> Box<SendAbilityChecker + Send> {
        Box::from(ProtocolBasedSendAbilityChecker::new(self.get_address()))
    }

    fn recv(&mut self) -> Option<BinMsg> {
        let mut buff = [0u8; 65536];
        match self.socket.recv_from(&mut buff) {
            Ok((len, addr)) => {
                let maddr = convert_address(addr);
                Some(BinMsg::new(Vec::from(&buff[..len]), maddr))
            }
            Err(_) => None,
        }

    }

    fn send(&mut self, msg: BinMsg) -> bool {
        unimplemented!();
    }

    fn serialize(&self, msg: Msg) -> BinMsg {
        unimplemented!();
    }

    fn parse(&self, binmsg: BinMsg) -> Msg {
        unimplemented!();
    }

    fn get_address(&self) -> Multiaddr {
        convert_address(self.socket.local_addr().expect(
            "UDPGateway: Did not bind to any address.",
        ))
    }
}

fn convert_address(addr: SocketAddr) -> Multiaddr {
    match addr {
        SocketAddr::V4(a) => Multiaddr::new(&format!("/ip4/{}/udp/{}", a.ip(), a.port())),
        SocketAddr::V6(a) => Multiaddr::new(&format!("/ip6/{}/udp/{}", a.ip(), a.port())),
    }.expect("UDPGateway: Error parsing address.")
}


#[cfg(test)]
mod tests {

    use std::thread;
    use std::net::UdpSocket;
    use gateway::udp::UdpGateway;
    use gateway::MsgGateway;

    #[test]
    fn test_udp_recv() {
        let socket = UdpSocket::bind("127.0.0.9:12345").unwrap();
        let mut gw = UdpGateway::new("127.0.0.2:12345");

        thread::spawn(move || {
            let _ = socket
                .send_to(&(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]), "127.0.0.2:12345")
                .expect("Failed to send data...");
        });

        let msg = gw.recv().unwrap();
        assert_eq!(msg.get_addr().to_string(), "/ip4/127.0.0.9/udp/12345");
        assert_eq!(
            msg.get_payload(),
            &vec![1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8]
        );
    }
}
