use std::sync::Arc;

use cornercase_core::error::Error;
use cornercase_core::secrets::Secrets;
use keyring_core::{CredentialStore, Entry};

const SERVICE: &str = "dev.usecornercase.mobile";

pub(crate) struct KeychainSecrets {
    store: Arc<CredentialStore>,
}

impl KeychainSecrets {
    pub(crate) fn new(store: Arc<CredentialStore>) -> Self {
        Self { store }
    }

    fn entry(&self, key: &str) -> Result<Entry, Error> {
        self.store.build(SERVICE, key, None).map_err(|error| secrets_error(&error))
    }
}

impl Secrets for KeychainSecrets {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        match self.entry(key)?.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(error) => Err(secrets_error(&error)),
        }
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.entry(key)?.set_secret(value).map_err(|error| secrets_error(&error))
    }

    fn delete(&self, key: &str) -> Result<(), Error> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(error) => Err(secrets_error(&error)),
        }
    }
}

fn secrets_error(error: &keyring_core::Error) -> Error {
    match error {
        keyring_core::Error::Ambiguous(entries) => {
            Error::Secrets(format!("the keychain holds {} items for one key", entries.len()))
        }
        error => Error::Secrets(format!("{error}")),
    }
}

#[cfg(test)]
mod tests;
