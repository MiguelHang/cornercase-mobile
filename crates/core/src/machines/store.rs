use std::fs::{self, File};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::error::Category;

use super::Machine;
use crate::error::Error;

const VERSION: u32 = 1;

pub trait MachineStore: Send + Sync {
    fn load(&self) -> Result<Saved, Error>;
    fn save(&self, saved: &Saved) -> Result<(), Error>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Saved {
    pub next_id: u64,
    pub machines: Vec<Machine>,
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
}

#[derive(Deserialize)]
struct Version {
    version: u32,
}

#[derive(Deserialize)]
struct Read {
    next_id: u64,
    machines: Vec<Machine>,
}

impl JsonFileStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn temporary(&self) -> PathBuf {
        let name = self.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        self.path.with_file_name(format!(".{name}.tmp"))
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
        let written = Written { version: VERSION, next_id: saved.next_id, machines: &saved.machines };
        let text = serde_json::to_vec_pretty(&written).map_err(|error| unreadable(&error))?;
        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder).map_err(|error| io_error("cannot create the folder", &error))?;
        }
        let temporary = self.temporary();
        or_remove(write(&temporary, &text), &temporary)
            .map_err(|error| io_error("cannot write the temporary file", &error))?;
        or_remove(fs::rename(&temporary, &self.path), &temporary)
            .map_err(|error| io_error("cannot replace the file", &error))
    }
}

fn parsed(text: &[u8]) -> Result<Saved, Error> {
    let Version { version } = serde_json::from_slice(text).map_err(|error| unreadable(&error))?;
    if version != VERSION {
        return Err(Error::Store(format!("the file's format version {version} is unknown")));
    }
    let Read { next_id, machines } = serde_json::from_slice(text).map_err(|error| unreadable(&error))?;
    Ok(Saved { next_id, machines })
}

fn write(path: &Path, text: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(text)?;
    file.sync_all()
}

fn or_remove<T>(result: io::Result<T>, temporary: &Path) -> io::Result<T> {
    if result.is_err() {
        fs::remove_file(temporary).ok();
    }
    result
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
