use serde::{Deserialize, Serialize};

use self::route::{Address, Route};

pub(crate) mod name;
pub mod pick;
pub mod registry;
pub mod route;
pub mod same;
pub mod store;

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

#[cfg(test)]
mod tests;
