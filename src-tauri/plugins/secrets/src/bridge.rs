use cornercase_core::error::Error;
use cornercase_core::secrets::{Secrets, hex};

const CANNOT_READ: &str = "cannot read a secret from the keystore";
const CANNOT_WRITE: &str = "cannot write a secret to the keystore";
const CANNOT_DELETE: &str = "cannot delete a secret from the keystore";
const NOT_HEX: &str = "the keystore returned a secret that is not hex";

pub(crate) trait Commands: Send + Sync {
    fn get(&self, name: &str) -> Result<Option<String>, String>;
    fn set(&self, name: &str, value: &str) -> Result<(), String>;
    fn delete(&self, name: &str) -> Result<(), String>;
}

pub(crate) struct Bridge<C>(C);

impl<C: Commands> Bridge<C> {
    pub(crate) fn new(commands: C) -> Self {
        Self(commands)
    }
}

impl<C: Commands> Secrets for Bridge<C> {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let value = self.0.get(key).map_err(|reason| refused(CANNOT_READ, &reason))?;
        value.map(|text| hex::decoded(&text).map_err(|_| Error::Secrets(NOT_HEX.to_owned()))).transpose()
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.0.set(key, &hex::encoded(value)).map_err(|reason| refused(CANNOT_WRITE, &reason))
    }

    fn delete(&self, key: &str) -> Result<(), Error> {
        self.0.delete(key).map_err(|reason| refused(CANNOT_DELETE, &reason))
    }
}

fn refused(operation: &str, reason: &str) -> Error {
    Error::Secrets(format!("{operation}: {reason}"))
}

#[cfg(test)]
mod tests;
