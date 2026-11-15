use std::collections::HashMap;
use std::fmt;
use std::hash::BuildHasher;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::name::checked_name;
use super::route::{Address, Route, Token};
use super::same::{Same, same_machine};
use super::store::{Loaded, MachineStore, Saved};
use super::{Machine, MachineId};
use crate::error::Error;
use crate::secrets::Secrets;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Merged {
    pub added: Vec<Address>,
    pub token_updated: bool,
    pub skipped: Vec<Address>,
}

#[derive(Default)]
struct Sorted {
    new: Vec<Route>,
    changed: Vec<Route>,
    skipped: Vec<Address>,
}

pub struct Machines {
    store: Box<dyn MachineStore>,
    secrets: Arc<dyn Secrets>,
    saved: Saved,
}

impl Machines {
    pub fn load(store: Box<dyn MachineStore>, secrets: Arc<dyn Secrets>) -> Result<(Self, Vec<MachineId>), Error> {
        let Loaded { mut saved, dropped } = store.load()?;
        let after_last = saved.machines.iter().map(|machine| machine.id.0.saturating_add(1)).max().unwrap_or_default();
        saved.next_id = saved.next_id.max(after_last);
        let forgotten = if dropped.is_empty() {
            Vec::new()
        } else {
            forget(store.as_ref(), secrets.as_ref(), &mut saved, &dropped)?
        };
        for machine in &mut saved.machines {
            fill_tokens(secrets.as_ref(), machine)?;
        }
        Ok((Self { store, secrets, saved }, forgotten))
    }

    pub fn machines(&self) -> &[Machine] {
        &self.saved.machines
    }

    pub fn machine(&self, id: MachineId) -> Option<&Machine> {
        self.saved.machines.iter().find(|machine| machine.id == id)
    }

    pub fn same<S: BuildHasher>(
        &self,
        routes: &[Route],
        name: &str,
        probed_boot: Option<&str>,
        seen_boots: &HashMap<MachineId, String, S>,
    ) -> Option<Same> {
        same_machine(self.machines(), routes, name, probed_boot, seen_boots)
    }

    pub fn add(&mut self, name: &str, routes: &[Route]) -> Result<MachineId, Error> {
        if routes.is_empty() {
            return Err(Error::NoToken);
        }
        let name = checked_name(name, self.machines(), None)?;
        let id = MachineId(self.saved.next_id);
        let routes = routes.to_vec();
        self.write_tokens(id, &routes)?;
        let mut saved = self.saved.clone();
        saved.next_id += 1;
        saved.machines.push(Machine { id, name, routes, last_route: None });
        self.commit(saved)?;
        Ok(id)
    }

    pub fn merge(&mut self, id: MachineId, routes: &[Route]) -> Result<Merged, Error> {
        let Sorted { new, mut changed, skipped } = self.sorted(id, routes.to_vec())?;
        changed.extend(self.with_new_secret(id, routes));
        let merged = Merged { added: addresses(&new), token_updated: !changed.is_empty(), skipped };
        let routes = [new, changed].concat();
        if !routes.is_empty() {
            self.write_tokens(id, &routes)?;
            self.save_routes(id, routes)?;
        }
        Ok(merged)
    }

    pub fn learn(&mut self, id: MachineId, routes: &[Route]) -> Result<Vec<Route>, Error> {
        let new = self.sorted(id, routes.to_vec())?.new;
        if !new.is_empty() {
            self.write_tokens(id, &new)?;
            self.save_routes(id, new.clone())?;
        }
        Ok(new)
    }

    pub fn rename(&mut self, id: MachineId, name: &str) -> Result<(), Error> {
        let current = self.machine(id).ok_or(Error::UnknownMachine)?;
        let name = checked_name(name, self.machines(), Some(id))?;
        if current.name == name {
            return Ok(());
        }
        let mut saved = self.saved.clone();
        find(&mut saved, id)?.name = name;
        self.commit(saved)
    }

    pub fn remove(&mut self, id: MachineId) -> Result<(), Error> {
        let mut saved = self.saved.clone();
        let Some(index) = saved.machines.iter().position(|machine| machine.id == id) else {
            return Ok(());
        };
        let removed = saved.machines.remove(index);
        if saved.active == Some(id) {
            saved.active = None;
        }
        self.commit(saved)?;
        removed.routes.iter().map(|route| self.secrets.delete(&key(id, route.address()))).fold(Ok(()), Result::and)
    }

    pub fn remove_route(&mut self, id: MachineId, address: &Address) -> Result<(), Error> {
        let mut saved = self.saved.clone();
        let machine = find(&mut saved, id)?;
        let index = position(&machine.routes, address)?;
        if machine.routes.len() == 1 {
            return Err(Error::LastRoute);
        }
        machine.routes.remove(index);
        if machine.last_route.as_ref() == Some(address) {
            machine.last_route = None;
        }
        self.commit(saved)?;
        self.secrets.delete(&key(id, address))
    }

    pub fn move_route(&mut self, id: MachineId, address: &Address, direction: Direction) -> Result<(), Error> {
        let mut saved = self.saved.clone();
        let routes = &mut find(&mut saved, id)?.routes;
        let index = position(routes, address)?;
        let neighbour = match direction {
            Direction::Up => index.checked_sub(1),
            Direction::Down => Some(index + 1).filter(|next| *next < routes.len()),
        };
        let Some(neighbour) = neighbour else {
            return Ok(());
        };
        routes.swap(index, neighbour);
        self.commit(saved)
    }

    pub fn remember(&mut self, id: MachineId, address: &Address) -> Result<(), Error> {
        let machine = self.machine(id).ok_or(Error::UnknownMachine)?;
        if machine.last_route.as_ref() == Some(address) {
            return Ok(());
        }
        if !machine.routes.iter().any(|route| route.address() == address) {
            return Err(Error::UnknownRoute);
        }
        let mut saved = self.saved.clone();
        find(&mut saved, id)?.last_route = Some(address.clone());
        self.commit(saved)
    }

    pub fn active(&self) -> Option<MachineId> {
        let saved = self.saved.active.filter(|id| self.machine(*id).is_some());
        saved.or_else(|| self.saved.machines.first().map(|machine| machine.id))
    }

    pub fn activate(&mut self, id: MachineId) -> Result<(), Error> {
        self.machine(id).ok_or(Error::UnknownMachine)?;
        if self.saved.active == Some(id) {
            return Ok(());
        }
        let mut saved = self.saved.clone();
        saved.active = Some(id);
        self.commit(saved)
    }

    fn sorted(&self, id: MachineId, routes: Vec<Route>) -> Result<Sorted, Error> {
        let machine = self.machine(id).ok_or(Error::UnknownMachine)?;
        let elsewhere = |address: &Address| {
            self.machines()
                .iter()
                .any(|other| other.id != id && other.routes.iter().any(|saved| saved.address() == address))
        };
        let mut sorted = Sorted::default();
        for route in routes {
            match machine.routes.iter().find(|saved| saved.address() == route.address()) {
                Some(saved) if saved == &route => {}
                Some(_) => sorted.changed.push(route),
                None if elsewhere(route.address()) => sorted.skipped.push(route.address().clone()),
                None if sorted.new.iter().any(|known| known.address() == route.address()) => {}
                None => sorted.new.push(route),
            }
        }
        Ok(sorted)
    }

    fn with_new_secret(&self, id: MachineId, routes: &[Route]) -> Vec<Route> {
        let secret = routes.first().and_then(Route::token);
        let (Some(secret), Some(machine)) = (secret, self.machine(id)) else {
            return Vec::new();
        };
        machine
            .routes
            .iter()
            .filter(|saved| saved.token() != Some(secret))
            .filter(|saved| routes.iter().all(|route| route.address() != saved.address()))
            .map(|saved| Route::cli(saved.address().clone(), Some(secret.clone())))
            .collect()
    }

    fn write_tokens(&self, id: MachineId, routes: &[Route]) -> Result<(), Error> {
        for route in routes {
            let token = route.token().ok_or(Error::NoToken)?;
            self.secrets.set(&key(id, route.address()), token.as_str().as_bytes())?;
        }
        Ok(())
    }

    fn save_routes(&mut self, id: MachineId, routes: Vec<Route>) -> Result<(), Error> {
        let mut saved = self.saved.clone();
        let saved_routes = &mut find(&mut saved, id)?.routes;
        for route in routes {
            match saved_routes.iter_mut().find(|existing| existing.address() == route.address()) {
                Some(existing) => *existing = route,
                None => saved_routes.push(route),
            }
        }
        self.commit(saved)
    }

    fn commit(&mut self, saved: Saved) -> Result<(), Error> {
        self.store.save(&saved)?;
        self.saved = saved;
        Ok(())
    }
}

impl fmt::Debug for Machines {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Machines").field("machines", &self.saved.machines).finish_non_exhaustive()
    }
}

fn forget(
    store: &dyn MachineStore,
    secrets: &dyn Secrets,
    saved: &mut Saved,
    dropped: &[(MachineId, Address)],
) -> Result<Vec<MachineId>, Error> {
    for (id, address) in dropped {
        if !saved.machines.iter().any(|machine| machine.id == *id && position(&machine.routes, address).is_ok()) {
            secrets.delete(&key(*id, address))?;
        }
    }
    let forgotten: Vec<MachineId> =
        saved.machines.iter().filter(|machine| machine.routes.is_empty()).map(|machine| machine.id).collect();
    saved.machines.retain(|machine| !machine.routes.is_empty());
    if saved.active.is_some_and(|id| forgotten.contains(&id)) {
        saved.active = None;
    }
    store.save(saved)?;
    Ok(forgotten)
}

fn fill_tokens(secrets: &dyn Secrets, machine: &mut Machine) -> Result<(), Error> {
    for route in &mut machine.routes {
        let value = secrets.get(&key(machine.id, route.address()))?;
        *route =
            Route::cli(route.address().clone(), value.and_then(|bytes| String::from_utf8(bytes).ok()).map(Token::new));
    }
    Ok(())
}

fn find(saved: &mut Saved, id: MachineId) -> Result<&mut Machine, Error> {
    saved.machines.iter_mut().find(|machine| machine.id == id).ok_or(Error::UnknownMachine)
}

fn addresses(routes: &[Route]) -> Vec<Address> {
    routes.iter().map(|route| route.address().clone()).collect()
}

fn position(routes: &[Route], address: &Address) -> Result<usize, Error> {
    routes.iter().position(|route| route.address() == address).ok_or(Error::UnknownRoute)
}

fn key(id: MachineId, address: &Address) -> String {
    format!("route/{}/{address}", id.0)
}

#[cfg(test)]
mod tests;
