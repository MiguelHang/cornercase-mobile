use std::fmt;
use std::sync::Arc;

use super::route::{Address, Route, Token};
use super::store::{MachineStore, Saved};
use super::{Machine, MachineId};
use crate::error::Error;
use crate::secrets::Secrets;

pub struct Machines {
    store: Box<dyn MachineStore>,
    secrets: Arc<dyn Secrets>,
    saved: Saved,
}

impl Machines {
    pub fn load(store: Box<dyn MachineStore>, secrets: Arc<dyn Secrets>) -> Result<Self, Error> {
        let mut saved = store.load()?;
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

    pub fn add(&mut self, name: String, route: Route) -> Result<MachineId, Error> {
        let id = MachineId(self.saved.next_id);
        self.write_token(id, &route)?;
        let mut saved = self.saved.clone();
        saved.next_id += 1;
        saved.machines.push(Machine { id, name, routes: vec![route], last_route: None });
        self.commit(saved)?;
        Ok(id)
    }

    pub fn add_route(&mut self, id: MachineId, route: Route) -> Result<(), Error> {
        self.machine(id).ok_or_else(unknown_machine)?;
        self.write_token(id, &route)?;
        let mut saved = self.saved.clone();
        let routes = &mut find(&mut saved, id)?.routes;
        match routes.iter_mut().find(|existing| existing.address() == route.address()) {
            Some(existing) => *existing = route,
            None => routes.push(route),
        }
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

    pub fn remember(&mut self, id: MachineId, address: &Address) -> Result<(), Error> {
        let machine = self.machine(id).ok_or_else(unknown_machine)?;
        if machine.last_route.as_ref() == Some(address) {
            return Ok(());
        }
        if !machine.routes.iter().any(|route| route.address() == address) {
            return Err(Error::Store("the machine has no route at that address".to_owned()));
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
    saved.machines.iter_mut().find(|machine| machine.id == id).ok_or_else(unknown_machine)
}

fn unknown_machine() -> Error {
    Error::Store("no saved machine has that id".to_owned())
}

fn key(id: MachineId, address: &Address) -> String {
    format!("route/{}/{address}", id.0)
}

#[cfg(test)]
mod tests;
