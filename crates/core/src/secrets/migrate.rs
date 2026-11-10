use super::Secrets;
use super::file::FileSecrets;
use crate::error::Error;

#[derive(Debug, PartialEq, Eq)]
pub enum Migration {
    NoFile,
    Moved(usize),
}

pub fn migrate(file: &FileSecrets, store: &dyn Secrets) -> Result<Migration, Error> {
    let Some(values) = file.values()? else {
        return Ok(Migration::NoFile);
    };
    for (key, value) in &values {
        store
            .set(key, value)
            .map_err(|error| Error::Secrets(format!("cannot copy the secrets file into the store: {error}")))?;
    }
    file.remove()?;
    Ok(Migration::Moved(values.len()))
}

#[cfg(test)]
mod tests;
