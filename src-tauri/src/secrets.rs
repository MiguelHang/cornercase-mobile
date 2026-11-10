use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use cornercase_core::error::Error;
use cornercase_core::secrets::Secrets;
#[cfg(target_os = "android")]
use cornercase_core::secrets::file::FileSecrets;
use keyring_core::{CredentialStore, Entry};

#[cfg(target_os = "ios")]
mod ios;

#[cfg(target_os = "ios")]
pub(crate) use ios::store;

const SERVICE: &str = "dev.usecornercase.mobile";

#[cfg(not(target_os = "ios"))]
const NO_STORE: &str = "no secret store on this platform yet";

#[cfg(target_os = "android")]
const ROUTES_FILE: &str = "route-secrets.json";

type Modifiers = HashMap<&'static str, &'static str>;

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

#[cfg(not(target_os = "ios"))]
pub(crate) fn store() -> KeychainSecrets {
    KeychainSecrets::new(Err(Error::Secrets(NO_STORE.to_owned())), Modifiers::new())
}

#[cfg(target_os = "android")]
pub(crate) fn routes(data_dir: Option<&Path>) -> Arc<dyn Secrets> {
    match data_dir {
        Some(folder) => Arc::new(FileSecrets::new(folder.join(ROUTES_FILE))),
        None => Arc::new(store()),
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) fn routes(_: Option<&Path>) -> Arc<dyn Secrets> {
    Arc::new(store())
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
