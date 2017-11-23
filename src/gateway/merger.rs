use gateway::{MsgGateway, SendAbilityChecker};
use multiaddr::Multiaddr;
use msg::Msg;

pub struct MergerGateway {
    validators: Vec<Box<SendAbilityChecker>>,
    gateways: Vec<Box<MsgGateway>>,
    current_gw: usize,
}

impl MergerGateway {
    pub fn new() -> MergerGateway {
        MergerGateway {
            validators: Vec::new(),
            gateways: Vec::new(),
            current_gw: 0,
        }
    }

    pub fn register_gateway(&mut self, gateway: Box<MsgGateway>) {
        self.validators.push(gateway.get_send_ability_checker());
        self.gateways.push(gateway);

    }

    pub fn has_some_gateways(&self) -> bool {
        self.gateways.len() > 0
    }
}

impl MsgGateway for MergerGateway {
    fn send(&mut self, data: Msg) -> bool {
        for (i, val) in self.validators.iter().enumerate() {
            if val.can_send(&data.addr) {
                self.gateways[i].send(data);
                return true;
            }
        }
        eprintln!("Can't send {:?}", data);
        false
    }

    fn recv(&mut self) -> Option<Msg> {
        self.current_gw = (self.current_gw + 1) % self.gateways.len();
        self.gateways[self.current_gw].recv()
    }

    fn get_send_ability_checker(&self) -> Box<SendAbilityChecker + Send> {
        Box::from(MergeSendChecker {
            validators: self.gateways
                .iter()
                .map(|g| g.get_send_ability_checker())
                .collect(),
        })
    }
}

struct MergeSendChecker {
    validators: Vec<Box<SendAbilityChecker + Send>>,
}

impl SendAbilityChecker for MergeSendChecker {
    fn can_send(&self, m: &Multiaddr) -> bool {
        for val in self.validators.iter() {
            if val.can_send(&m) {
                return true;
            }
        }
        false
    }
}
