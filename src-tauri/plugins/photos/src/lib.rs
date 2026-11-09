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

pub use platform::Photos;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    platform::managing(Builder::new("photos")).build()
}

#[cfg(test)]
mod tests;
