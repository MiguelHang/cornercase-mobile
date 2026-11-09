use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use self::route::{Address, Route};

pub mod code;
pub(crate) mod name;
pub mod pick;
pub mod registry;
pub mod route;
pub mod same;
pub mod store;

const TAILSCALE_V4_FIRST: u8 = 100;
const TAILSCALE_V4_MASK: u8 = 0b1100_0000;
const TAILSCALE_V4_SECOND: u8 = 64;
const TAILSCALE_V6_PREFIX: [u16; 3] = [0xfd7a, 0x115c, 0xa1e0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MachineId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Machine {
    pub id: MachineId,
    pub name: String,
    pub routes: Vec<Route>,
    pub last_route: Option<Address>,
}

pub(crate) fn tailscale(address: &Address) -> bool {
    match address.host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            let [first, second, ..] = ip.octets();
            first == TAILSCALE_V4_FIRST && second & TAILSCALE_V4_MASK == TAILSCALE_V4_SECOND
        }
        Ok(IpAddr::V6(ip)) => ip.segments().starts_with(&TAILSCALE_V6_PREFIX),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests;
