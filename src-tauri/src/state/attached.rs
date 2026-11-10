use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::commands::error::CommandError;

pub(crate) const MAX_PER_PANE: usize = 4;
pub(crate) const MAX_BYTES: usize = 64 << 20;

#[derive(Default)]
pub(crate) struct AttachedImages(Mutex<Held>);

#[derive(Default)]
struct Held {
    last: u64,
    images: Vec<Attached>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Attached {
    pub(crate) id: u64,
    pub(crate) pane: u64,
    pub(crate) name: String,
    pub(crate) bytes: Vec<u8>,
}

impl AttachedImages {
    pub(crate) fn store(&self, pane: u64, name: String, bytes: Vec<u8>) -> Result<u64, CommandError> {
        let mut held = self.held();
        if held.images.iter().filter(|image| image.pane == pane).count() >= MAX_PER_PANE {
            return Err(CommandError::too_many_attached(MAX_PER_PANE));
        }
        if held.bytes() + bytes.len() > MAX_BYTES {
            return Err(CommandError::attached_too_large());
        }
        held.last += 1;
        let id = held.last;
        held.images.push(Attached { id, pane, name, bytes });
        Ok(id)
    }

    pub(crate) fn take(&self, pane: u64, id: u64) -> Option<Attached> {
        let mut held = self.held();
        let index = held.images.iter().position(|image| image.id == id && image.pane == pane)?;
        Some(held.images.remove(index))
    }

    pub(crate) fn discard(&self, ids: &[u64]) {
        self.held().images.retain(|image| !ids.contains(&image.id));
    }

    pub(crate) fn clear(&self) {
        self.held().images.clear();
    }

    fn held(&self) -> MutexGuard<'_, Held> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Held {
    fn bytes(&self) -> usize {
        self.images.iter().map(|image| image.bytes.len()).sum()
    }
}

#[cfg(test)]
mod tests;
