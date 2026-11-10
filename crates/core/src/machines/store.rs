use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::error::Category;

use super::{Machine, MachineId};
use crate::error::Error;
use crate::files::write_atomically;

const VERSION: u32 = 1;

pub trait MachineStore: Send + Sync {
    fn load(&self) -> Result<Saved, Error>;
    fn save(&self, saved: &Saved) -> Result<(), Error>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Saved {
    pub next_id: u64,
    pub machines: Vec<Machine>,
    pub active: Option<MachineId>,
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
    machines: Vec<Machine>,
    #[serde(default)]
    active: Option<MachineId>,
}

impl JsonFileStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl MachineStore for JsonFileStore {
    fn load(&self) -> Result<Saved, Error> {
        match fs::read(&self.path) {
            Ok(text) => parsed(&text),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Saved::default()),
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

fn parsed(text: &[u8]) -> Result<Saved, Error> {
    let Version { version } = serde_json::from_slice(text).map_err(|error| unreadable(&error))?;
    if version != VERSION {
        return Err(Error::Store(format!("the file's format version {version} is unknown")));
    }
    let Read { next_id, machines, active } = serde_json::from_slice(text).map_err(|error| unreadable(&error))?;
    Ok(Saved { next_id, machines, active })
}

fn io_error(what: &str, error: &io::Error) -> Error {
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
