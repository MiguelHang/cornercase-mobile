use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{HistoryEntry, PaneFile, VERSION, parsed, serialized, written};
use crate::error::Error;
use crate::files::write_atomically;
use crate::machines::MachineId;
use crate::machines::store::{io_error, versioned};

const MOST_PANES: usize = 50;
const INDEX: &str = "index.json";

pub trait HistoryStore: Send + Sync {
    fn load(&self, machine: MachineId, boot: &str, pane: u64) -> Result<Vec<HistoryEntry>, Error>;
    fn save(&self, machine: MachineId, boot: &str, pane: u64, entries: &[HistoryEntry]) -> Result<(), Error>;
    fn forget(&self, machine: MachineId) -> Result<(), Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonFolderStore {
    folder: PathBuf,
}

#[derive(Serialize)]
struct WrittenIndex<'a> {
    version: u32,
    boot: &'a str,
    panes: &'a [u64],
}

#[derive(Deserialize)]
struct ReadIndex {
    boot: String,
    panes: Vec<u64>,
}

impl JsonFolderStore {
    pub fn new(folder: PathBuf) -> Self {
        Self { folder }
    }

    fn machine(&self, machine: MachineId) -> PathBuf {
        self.folder.join(machine.0.to_string())
    }

    fn pane(&self, machine: MachineId, pane: u64) -> PathBuf {
        self.machine(machine).join(format!("{pane}.json"))
    }

    fn panes(&self, machine: MachineId, boot: &str) -> Result<Vec<u64>, Error> {
        let folder = self.machine(machine);
        let index = read(&folder.join(INDEX))?.and_then(|text| versioned::<ReadIndex>(&text, VERSION).ok());
        match index {
            Some(ReadIndex { boot: kept, panes }) if kept == boot => Ok(panes),
            _ => removed(fs::remove_dir_all(&folder)).map(|()| Vec::new()),
        }
    }
}

impl HistoryStore for JsonFolderStore {
    fn load(&self, machine: MachineId, boot: &str, pane: u64) -> Result<Vec<HistoryEntry>, Error> {
        if !self.panes(machine, boot)?.contains(&pane) {
            return Ok(Vec::new());
        }
        let file = read(&self.pane(machine, pane))?.and_then(|text| parsed(&text).ok());
        Ok(file.filter(|file| file.boot == boot).map(|PaneFile { entries, .. }| entries).unwrap_or_default())
    }

    fn save(&self, machine: MachineId, boot: &str, pane: u64, entries: &[HistoryEntry]) -> Result<(), Error> {
        let mut panes = self.panes(machine, boot)?;
        write(&self.pane(machine, pane), &written(boot, entries)?)?;
        panes.retain(|&kept| kept != pane);
        panes.insert(0, pane);
        let dropped = panes.split_off(panes.len().min(MOST_PANES));
        let index = serialized(&WrittenIndex { version: VERSION, boot, panes: &panes })?;
        write(&self.machine(machine).join(INDEX), &index)?;
        dropped.into_iter().try_for_each(|old| removed(fs::remove_file(self.pane(machine, old))))
    }

    fn forget(&self, machine: MachineId) -> Result<(), Error> {
        removed(fs::remove_dir_all(self.machine(machine)))
    }
}

fn read(path: &Path) -> Result<Option<Vec<u8>>, Error> {
    match fs::read(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error("cannot read the history", &error)),
    }
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    write_atomically(path, bytes).map_err(|error| Error::Store(error.to_string()))
}

fn removed(result: io::Result<()>) -> Result<(), Error> {
    match result {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(io_error("cannot remove the history", &error)),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests;
