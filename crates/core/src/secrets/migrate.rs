use super::Secrets;
use super::file::{FileSecrets, Values};
use crate::error::Error;

#[derive(Debug, PartialEq, Eq)]
pub enum Migration {
    NoFile,
    Moved(usize),
}

pub fn migrate(file: &FileSecrets, store: &dyn Secrets) -> Result<Migration, Error> {
    let Some(values) = file.values()? else {
        file.discard_temporary();
        return Ok(Migration::NoFile);
    };
    let moved = copied(&values, store).and_then(|()| file.remove());
    if moved.is_err() {
        forget(&values, store);
    }
    moved.map(|()| Migration::Moved(values.len()))
}

fn copied(values: &Values, store: &dyn Secrets) -> Result<(), Error> {
    for (key, value) in values {
        store
            .set(key, value)
            .map_err(|error| Error::Secrets(format!("cannot copy the secrets file into the store: {error}")))?;
    }
    Ok(())
}

fn forget(values: &Values, store: &dyn Secrets) {
    for (key, _) in values {
        store.delete(key).ok();
    }
}

#[cfg(test)]
mod tests;
