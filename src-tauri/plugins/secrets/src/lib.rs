use std::sync::Arc;

use cornercase_core::secrets::Secrets;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[cfg(target_os = "android")]
mod android;
#[cfg(any(test, target_os = "android"))]
mod bridge;
#[cfg(not(target_os = "android"))]
mod other;

#[cfg(target_os = "android")]
use android as platform;
#[cfg(not(target_os = "android"))]
use other as platform;

pub struct SecretStore(Arc<dyn Secrets>);

impl SecretStore {
    pub fn secrets(&self) -> Arc<dyn Secrets> {
        Arc::clone(&self.0)
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    platform::managing(Builder::new("secrets")).build()
}

#[cfg(all(test, not(target_os = "android")))]
mod tests;
