use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_keyboard);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    native(Builder::new("keyboard")).build()
}

#[cfg(target_os = "ios")]
fn native<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.setup(|_, api| {
        api.register_ios_plugin(init_plugin_keyboard)?;
        Ok(())
    })
}

#[cfg(not(target_os = "ios"))]
fn native<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder
}

#[cfg(test)]
mod tests;
