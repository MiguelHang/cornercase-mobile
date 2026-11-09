use std::collections::HashMap;
use std::hash::BuildHasher;

use super::name::folded;
use super::route::Route;
use super::{Machine, MachineId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Same {
    pub machine: MachineId,
    pub why: Why,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Address,
    Token,
    Boot,
    Name,
    AlreadySaved,
}

pub fn same_machine<S: BuildHasher>(
    machines: &[Machine],
    route: &Route,
    name: &str,
    probed_boot: Option<&str>,
    seen_boots: &HashMap<MachineId, String, S>,
) -> Option<Same> {
    by_address(machines, route)
        .or_else(|| by_token(machines, route))
        .or_else(|| by_boot(machines, probed_boot?, seen_boots))
        .or_else(|| by_name(machines, name))
}

fn by_address(machines: &[Machine], route: &Route) -> Option<Same> {
    machines.iter().find_map(|machine| {
        let saved = machine.routes.iter().find(|saved| saved.address() == route.address())?;
        let token = saved.token();
        let why = if token.is_some() && token == route.token() { Why::AlreadySaved } else { Why::Address };
        Some(Same { machine: machine.id, why })
    })
}

fn by_token(machines: &[Machine], route: &Route) -> Option<Same> {
    let token = route.token()?;
    first(machines, Why::Token, |machine| machine.routes.iter().any(|saved| saved.token() == Some(token)))
}

fn by_boot<S: BuildHasher>(
    machines: &[Machine],
    boot: &str,
    seen_boots: &HashMap<MachineId, String, S>,
) -> Option<Same> {
    first(machines, Why::Boot, |machine| seen_boots.get(&machine.id).is_some_and(|seen| seen == boot))
}

fn by_name(machines: &[Machine], name: &str) -> Option<Same> {
    let wanted = folded(name);
    first(machines, Why::Name, |machine| folded(&machine.name) == wanted)
}

fn first(machines: &[Machine], why: Why, matches: impl Fn(&Machine) -> bool) -> Option<Same> {
    machines.iter().find(|machine| matches(machine)).map(|machine| Same { machine: machine.id, why })
}

#[cfg(test)]
mod tests;
