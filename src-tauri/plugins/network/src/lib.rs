use cornercase_core::network::Network;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[cfg(target_os = "ios")]
mod ios;
#[cfg(not(target_os = "ios"))]
mod other;

#[cfg(target_os = "ios")]
use ios as platform;
#[cfg(not(target_os = "ios"))]
use other as platform;

pub fn init<R: Runtime>(on_path: impl Fn(Network) + Send + Sync + 'static) -> TauriPlugin<R> {
    platform::watching(Builder::new("network"), on_path).build()
}
