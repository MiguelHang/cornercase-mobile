use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::error::Category;

use super::route::{Address, Route};
use super::{Machine, MachineId};
use crate::error::Error;
use crate::files::write_atomically;

const VERSION: u32 = 1;

pub trait MachineStore: Send + Sync {
    fn load(&self) -> Result<Loaded, Error>;
    fn save(&self, saved: &Saved) -> Result<(), Error>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Saved {
    pub next_id: u64,
    pub machines: Vec<Machine>,
    pub active: Option<MachineId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Loaded {
    pub saved: Saved,
    pub dropped: Vec<(MachineId, Address)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonFileStore {
    path: PathBuf,
}

#[derive(Serialize)]
struct Written<'a> {
    version: u32,
    next_id: u64,
    machines: &'a [Machine],
    active: Option<MachineId>,
}

#[derive(Deserialize)]
struct Version {
    version: u32,
}

#[derive(Deserialize)]
struct Read {
    next_id: u64,
    machines: Vec<ReadMachine>,
    #[serde(default)]
    active: Option<MachineId>,
}

#[derive(Deserialize)]
struct ReadMachine {
    id: MachineId,
    name: String,
    routes: Vec<ReadRoute>,
    last_route: Option<Address>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ReadRoute {
    Bridge { url: Address },
    Cli { url: Address },
}

impl JsonFileStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl MachineStore for JsonFileStore {
    fn load(&self) -> Result<Loaded, Error> {
        match fs::read(&self.path) {
            Ok(text) => parsed(&text),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Loaded::default()),
            Err(error) => Err(io_error("cannot read the file", &error)),
        }
    }

    fn save(&self, saved: &Saved) -> Result<(), Error> {
        let written =
            Written { version: VERSION, next_id: saved.next_id, machines: &saved.machines, active: saved.active };
        let text = serde_json::to_vec_pretty(&written).map_err(|error| unreadable(&error))?;
        write_atomically(&self.path, &text).map_err(|error| Error::Store(error.to_string()))
    }
}

fn parsed(text: &[u8]) -> Result<Loaded, Error> {
    let Read { next_id, machines, active } = versioned(text, VERSION)?;
    let mut dropped = Vec::new();
    let machines = machines.into_iter().map(|machine| kept(machine, &mut dropped)).collect();
    Ok(Loaded { saved: Saved { next_id, machines, active }, dropped })
}

fn kept(machine: ReadMachine, dropped: &mut Vec<(MachineId, Address)>) -> Machine {
    let ReadMachine { id, name, routes, last_route } = machine;
    let mut kept = Vec::new();
    for route in routes {
        match route {
            ReadRoute::Bridge { url } => dropped.push((id, url)),
            ReadRoute::Cli { url } => kept.push(Route::cli(url, None)),
        }
    }
    let last_route = last_route.filter(|address| kept.iter().any(|route| route.address() == address));
    Machine { id, name, routes: kept, last_route }
}

pub(crate) fn versioned<T: DeserializeOwned>(text: &[u8], expected: u32) -> Result<T, Error> {
    let Version { version } = serde_json::from_slice(text).map_err(|error| unreadable(&error))?;
    if version != expected {
        return Err(Error::Store(format!("the file's format version {version} is unknown")));
    }
    serde_json::from_slice(text).map_err(|error| unreadable(&error))
}

pub(crate) fn io_error(what: &str, error: &io::Error) -> Error {
    Error::Store(format!("{what}: {error}"))
}

fn unreadable(error: &serde_json::Error) -> Error {
    let shape = match error.classify() {
        Category::Data => "does not hold the expected fields",
        Category::Syntax | Category::Eof | Category::Io => "is not JSON",
    };
    Error::Store(format!("the file {shape} (line {}, column {})", error.line(), error.column()))
}

#[cfg(test)]
mod tests;
