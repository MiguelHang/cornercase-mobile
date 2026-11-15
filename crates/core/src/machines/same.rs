use std::collections::HashMap;
use std::hash::BuildHasher;

use super::name::folded;
use super::route::{Address, Route};
use super::{Machine, MachineId, tailscale};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Same {
    pub machine: MachineId,
    pub why: Why,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    AlreadySaved,
    Tailscale,
    AllAddresses,
    Address,
    Boot,
    Name,
}

impl Why {
    pub fn merges(self) -> bool {
        matches!(self, Self::AlreadySaved | Self::Tailscale | Self::AllAddresses)
    }
}

pub fn same_machine<S: BuildHasher>(
    machines: &[Machine],
    routes: &[Route],
    name: &str,
    probed_boot: Option<&str>,
    seen_boots: &HashMap<MachineId, String, S>,
) -> Option<Same> {
    already_saved(machines, routes)
        .or_else(|| by_address(machines, routes, Why::Tailscale, tailscale))
        .or_else(|| by_all_addresses(machines, routes))
        .or_else(|| by_address(machines, routes, Why::Address, |_| true))
        .or_else(|| by_boot(machines, probed_boot?, seen_boots))
        .or_else(|| by_name(machines, name))
}

fn already_saved(machines: &[Machine], routes: &[Route]) -> Option<Same> {
    first(machines, Why::AlreadySaved, |machine| {
        !routes.is_empty() && routes.iter().all(|route| route.has_token() && machine.routes.contains(route))
    })
}

fn by_address(machines: &[Machine], routes: &[Route], why: Why, counts: impl Fn(&Address) -> bool) -> Option<Same> {
    first(machines, why, |machine| routes.iter().any(|route| counts(route.address()) && has(machine, route)))
}

fn by_all_addresses(machines: &[Machine], routes: &[Route]) -> Option<Same> {
    if routes.len() < 2 {
        return None;
    }
    first(machines, Why::AllAddresses, |machine| routes.iter().all(|route| has(machine, route)))
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

fn has(machine: &Machine, route: &Route) -> bool {
    machine.routes.iter().any(|saved| saved.address() == route.address())
}

fn first(machines: &[Machine], why: Why, matches: impl Fn(&Machine) -> bool) -> Option<Same> {
    machines.iter().find(|machine| matches(machine)).map(|machine| Same { machine: machine.id, why })
}

#[cfg(test)]
mod tests;
