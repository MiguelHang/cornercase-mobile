use std::collections::HashMap;
use std::sync::Arc;

use cornercase_core::error::Error;
use cornercase_core::secrets::Secrets;
use keyring_core::{CredentialStore, Entry};

#[cfg(target_os = "ios")]
pub(crate) mod ios;

const SERVICE: &str = "dev.usecornercase.mobile";

pub(crate) type Modifiers = HashMap<&'static str, &'static str>;

pub(crate) struct KeychainSecrets {
    store: Result<Arc<CredentialStore>, Error>,
    modifiers: Modifiers,
}

impl KeychainSecrets {
    pub(crate) fn new(store: Result<Arc<CredentialStore>, Error>, modifiers: Modifiers) -> Self {
        Self { store, modifiers }
    }

    fn entry(&self, key: &str) -> Result<Entry, Error> {
        let store = self.store.as_ref().map_err(Clone::clone)?;
        store.build(SERVICE, key, Some(&self.modifiers)).map_err(|error| secrets_error(&error))
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
pub(crate) fn mock_store() -> Arc<CredentialStore> {
    keyring_core::mock::Store::new().expect("a mock store")
}

#[cfg(test)]
pub(crate) fn on_mock(store: Arc<CredentialStore>) -> KeychainSecrets {
    KeychainSecrets::new(Ok(store), Modifiers::new())
}

#[cfg(test)]
mod tests;
