use std::path::Path;
use std::sync::Arc;

#[cfg(any(test, not(target_os = "ios")))]
use cornercase_core::error::Error;
use cornercase_core::secrets::Secrets;
#[cfg(any(test, target_os = "android"))]
use cornercase_core::secrets::file::FileSecrets;
#[cfg(any(test, target_os = "android"))]
use cornercase_core::secrets::migrate::{Migration, migrate};
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

#[cfg(any(test, target_os = "android"))]
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
        Some(folder) => moved(FileSecrets::new(folder.join(ROUTES_FILE)), store),
        None => store,
    }
}

#[cfg(any(test, target_os = "android"))]
fn moved(file: FileSecrets, store: Arc<dyn Secrets>) -> Arc<dyn Secrets> {
    let migration = migrate(&file, &*store);
    if let Some(line) = line(&migration) {
        eprintln!("{line}");
    }
    if migration.is_ok() { store } else { Arc::new(file) }
}

#[cfg(any(test, target_os = "android"))]
fn line(migration: &Result<Migration, Error>) -> Option<String> {
    match migration {
        Ok(Migration::NoFile) => None,
        Ok(Migration::Moved(count)) => Some(format!("cornercase-mobile: moved {count} route secrets into the store")),
        Err(error) => Some(format!("cornercase-mobile: the route secrets stay in their file: {error}")),
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) fn routes(_: Option<&Path>, store: Arc<dyn Secrets>) -> Arc<dyn Secrets> {
    store
}

#[cfg(test)]
mod tests;
