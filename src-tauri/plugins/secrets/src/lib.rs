use std::sync::Arc;

use cornercase_core::secrets::Secrets;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[cfg(test)]
mod bridge;
mod other;

pub struct SecretStore(Arc<dyn Secrets>);

impl SecretStore {
    pub fn secrets(&self) -> Arc<dyn Secrets> {
        Arc::clone(&self.0)
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    other::managing(Builder::new("secrets")).build()
}

#[cfg(test)]
mod tests;
