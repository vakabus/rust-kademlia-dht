use gateway::{MsgGateway, BinMsg, SendAbilityChecker};
use multiaddr::Multiaddr;

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
    fn send(&mut self, data: BinMsg) -> bool {
        for (i, val) in self.validators.iter().enumerate() {
            if val.can_send(data.get_addr()) {
                self.gateways[i].send(data);
                return true;
            }
        }
        eprintln!("Can't send {:?}", data);
        false
    }

    fn recv(&mut self) -> Option<BinMsg> {
        self.current_gw = (self.current_gw + 1) % self.gateways.len();
        self.gateways[self.current_gw].recv()
    }

    fn get_address(&self) -> Multiaddr {
        unimplemented!();
    }

    fn get_send_ability_checker(&self) -> Box<SendAbilityChecker + Send> {
        unimplemented!();
    }
}
