use cornercase_core::network::Network;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[cfg(not(target_os = "ios"))]
mod desktop;
#[cfg(target_os = "ios")]
mod mobile;

#[cfg(not(target_os = "ios"))]
use desktop as platform;
#[cfg(target_os = "ios")]
use mobile as platform;

pub fn init<R: Runtime>(on_path: impl Fn(Network) + Send + Sync + 'static) -> TauriPlugin<R> {
    platform::watching(Builder::new("network"), on_path).build()
}
