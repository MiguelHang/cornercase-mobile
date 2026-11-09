use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::Secrets;
use crate::error::Error;

#[derive(Default)]
pub(crate) struct MemorySecrets {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    values: HashMap<String, Vec<u8>>,
    failing: bool,
}

impl MemorySecrets {
    pub(crate) fn fail_writes(&self, failing: bool) {
        self.state().failing = failing;
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn writable(&self) -> Result<MutexGuard<'_, State>, Error> {
        let state = self.state();
        if state.failing {
            return Err(Error::Secrets("the memory store refuses writes".to_owned()));
        }
        Ok(state)
    }
}

impl Secrets for MemorySecrets {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.state().values.get(key).cloned())
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.writable()?.values.insert(key.to_owned(), value.to_vec());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), Error> {
        self.writable()?.values.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
