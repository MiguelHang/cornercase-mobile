use cornercase_core::network::Network;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod other;

#[cfg(any(target_os = "ios", target_os = "android"))]
use mobile as platform;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
use other as platform;

pub fn init<R: Runtime>(on_path: impl Fn(Network) + Send + Sync + 'static) -> TauriPlugin<R> {
    platform::watching(Builder::new("network"), on_path).build()
}
