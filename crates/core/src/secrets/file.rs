use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::error::Category;

use super::Secrets;
use crate::error::Error;
use crate::files::write_atomically;

const VERSION: u32 = 1;

type Entries = BTreeMap<String, String>;

#[derive(Debug)]
pub struct FileSecrets {
    path: PathBuf,
    writes: Mutex<()>,
}

#[derive(Serialize)]
struct Written<'a> {
    version: u32,
    secrets: &'a Entries,
}

#[derive(Deserialize)]
struct Stored {
    version: u32,
    #[serde(default)]
    secrets: Value,
}

impl FileSecrets {
    pub fn new(path: PathBuf) -> Self {
        Self { path, writes: Mutex::new(()) }
    }

    fn entries(&self) -> Result<Entries, Error> {
        match fs::read(&self.path) {
            Ok(text) => entries_in(&text),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Entries::new()),
            Err(error) => Err(Error::Secrets(format!("cannot read the secrets file: {error}"))),
        }
    }

    fn change(&self, edit: impl FnOnce(&mut Entries) -> bool) -> Result<(), Error> {
        let _writing = self.writes.lock().unwrap_or_else(PoisonError::into_inner);
        let mut entries = self.entries()?;
        if !edit(&mut entries) {
            return Ok(());
        }
        let text = serde_json::to_vec_pretty(&Written { version: VERSION, secrets: &entries })
            .map_err(|error| Error::Secrets(format!("cannot encode the secrets: {error}")))?;
        write_atomically(&self.path, &text).map_err(|error| Error::Secrets(error.to_string()))
    }
}

impl Secrets for FileSecrets {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.entries()?.get(key).map(|hex| decoded(hex)).transpose()
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.change(|entries| {
            entries.insert(key.to_owned(), encoded(value));
            true
        })
    }

    fn delete(&self, key: &str) -> Result<(), Error> {
        self.change(|entries| entries.remove(key).is_some())
    }
}

fn entries_in(text: &[u8]) -> Result<Entries, Error> {
    let stored: Stored = serde_json::from_slice(text).map_err(|error| shapeless(&error))?;
    if stored.version != VERSION {
        return Err(Error::Secrets(format!("the secrets file's format version {} is unknown", stored.version)));
    }
    serde_json::from_value(stored.secrets).map_err(|error| shapeless(&error))
}

fn shapeless(error: &serde_json::Error) -> Error {
    match error.classify() {
        Category::Data => Error::Secrets("the secrets file does not hold the expected fields".to_owned()),
        Category::Syntax | Category::Eof | Category::Io => {
            Error::Secrets(format!("the secrets file is not JSON (line {}, column {})", error.line(), error.column()))
        }
    }
}

fn encoded(value: &[u8]) -> String {
    value.iter().fold(String::with_capacity(value.len() * 2), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn decoded(hex: &str) -> Result<Vec<u8>, Error> {
    let (pairs, rest) = hex.as_bytes().as_chunks::<2>();
    let bytes: Option<Vec<u8>> = pairs.iter().copied().map(byte).collect();
    bytes
        .filter(|_| rest.is_empty())
        .ok_or_else(|| Error::Secrets("a secret in the secrets file is not hex".to_owned()))
}

fn byte([high, low]: [u8; 2]) -> Option<u8> {
    Some((nibble(high)? << 4) | nibble(low)?)
}

fn nibble(digit: u8) -> Option<u8> {
    char::from(digit).to_digit(16).and_then(|value| u8::try_from(value).ok())
}

#[cfg(test)]
mod tests;
