use std::sync::Arc;

use apple_native_keyring_store::protected::Store;
use keyring_core::CredentialStore;

use super::{KeychainSecrets, Modifiers, secrets_error};

pub(crate) fn store() -> KeychainSecrets {
    let store = Store::new().map(|store| -> Arc<CredentialStore> { store }).map_err(|error| secrets_error(&error));
    KeychainSecrets::new(store, Modifiers::from([("access-policy", "AfterFirstUnlockThisDeviceOnly")]))
}
