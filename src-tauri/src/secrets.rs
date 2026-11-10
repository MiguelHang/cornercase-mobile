use std::path::Path;
use std::sync::Arc;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
use cornercase_core::error::Error;
use cornercase_core::secrets::Secrets;
#[cfg(target_os = "android")]
use cornercase_core::secrets::file::FileSecrets;
#[cfg(target_os = "android")]
use tauri::Manager;
use tauri::{AppHandle, Runtime};
#[cfg(target_os = "android")]
use tauri_plugin_secrets::SecretStore;

#[cfg(test)]
pub(crate) use self::keychain::{KeychainSecrets, mock_store, on_mock};

#[cfg(any(test, not(target_os = "android")))]
mod keychain;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
const NO_STORE: &str = "no secret store on this platform yet";

#[cfg(target_os = "android")]
const ROUTES_FILE: &str = "route-secrets.json";

#[cfg(target_os = "android")]
pub(crate) fn store<R: Runtime>(app: &AppHandle<R>) -> Arc<dyn Secrets> {
    app.state::<SecretStore>().secrets()
}

#[cfg(target_os = "ios")]
pub(crate) fn store<R: Runtime>(_: &AppHandle<R>) -> Arc<dyn Secrets> {
    Arc::new(keychain::ios::store())
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub(crate) fn store<R: Runtime>(_: &AppHandle<R>) -> Arc<dyn Secrets> {
    Arc::new(keychain::KeychainSecrets::new(Err(Error::Secrets(NO_STORE.to_owned())), keychain::Modifiers::new()))
}

#[cfg(target_os = "android")]
pub(crate) fn routes(data_dir: Option<&Path>, store: Arc<dyn Secrets>) -> Arc<dyn Secrets> {
    match data_dir {
        Some(folder) => Arc::new(FileSecrets::new(folder.join(ROUTES_FILE))),
        None => store,
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) fn routes(_: Option<&Path>, store: Arc<dyn Secrets>) -> Arc<dyn Secrets> {
    store
}

#[cfg(test)]
mod tests;
