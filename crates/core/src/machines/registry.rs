use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::name::checked_name;
use super::route::{Address, Route, Token};
use super::store::{MachineStore, Saved};
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

pub struct Machines {
    store: Box<dyn MachineStore>,
    secrets: Arc<dyn Secrets>,
    saved: Saved,
}

impl Machines {
    pub fn load(store: Box<dyn MachineStore>, secrets: Arc<dyn Secrets>) -> Result<Self, Error> {
        let mut saved = store.load()?;
        let after_last = saved.machines.iter().map(|machine| machine.id.0.saturating_add(1)).max().unwrap_or_default();
        saved.next_id = saved.next_id.max(after_last);
        for machine in &mut saved.machines {
            fill_tokens(secrets.as_ref(), machine)?;
        }
        Ok(Self { store, secrets, saved })
    }

    pub fn machines(&self) -> &[Machine] {
        &self.saved.machines
    }

    pub fn machine(&self, id: MachineId) -> Option<&Machine> {
        self.saved.machines.iter().find(|machine| machine.id == id)
    }

    pub fn add(&mut self, name: &str, route: Route) -> Result<MachineId, Error> {
        let name = checked_name(name, self.machines(), None)?;
        let id = MachineId(self.saved.next_id);
        self.write_token(id, &route)?;
        let mut saved = self.saved.clone();
        saved.next_id += 1;
        saved.machines.push(Machine { id, name, routes: vec![route], last_route: None });
        self.commit(saved)?;
        Ok(id)
    }

    pub fn add_route(&mut self, id: MachineId, route: Route) -> Result<(), Error> {
        self.machine(id).ok_or(Error::UnknownMachine)?;
        self.write_token(id, &route)?;
        let mut saved = self.saved.clone();
        let routes = &mut find(&mut saved, id)?.routes;
        match routes.iter_mut().find(|existing| existing.address() == route.address()) {
            Some(existing) => *existing = route,
            None => routes.push(route),
        }
        self.commit(saved)
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

    fn write_token(&self, id: MachineId, route: &Route) -> Result<(), Error> {
        let token = route.token().ok_or(Error::NoToken)?;
        self.secrets.set(&key(id, route.address()), token.as_str().as_bytes())
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

fn fill_tokens(secrets: &dyn Secrets, machine: &mut Machine) -> Result<(), Error> {
    for route in &mut machine.routes {
        let value = secrets.get(&key(machine.id, route.address()))?;
        route.set_token(value.and_then(|bytes| String::from_utf8(bytes).ok()).map(Token::new));
    }
    Ok(())
}

fn find(saved: &mut Saved, id: MachineId) -> Result<&mut Machine, Error> {
    saved.machines.iter_mut().find(|machine| machine.id == id).ok_or(Error::UnknownMachine)
}

fn position(routes: &[Route], address: &Address) -> Result<usize, Error> {
    routes.iter().position(|route| route.address() == address).ok_or(Error::UnknownRoute)
}

fn key(id: MachineId, address: &Address) -> String {
    format!("route/{}/{address}", id.0)
}

#[cfg(test)]
mod tests;
