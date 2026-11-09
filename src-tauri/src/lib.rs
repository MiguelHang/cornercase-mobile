use std::sync::Arc;

use tauri::Manager;

use crate::state::AppState;

mod bindings;
mod commands;
mod events;
mod identity;
mod lifecycle;
mod network;
mod secrets;
mod state;
#[cfg(test)]
mod test_util;
mod view;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta = bindings::specta();
    let (hear, paths) = network::listen();
    tauri::Builder::default()
        .plugin(tauri_plugin_network::init(hear))
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            specta.mount_events(app);
            app.manage(AppState::load(app.path().app_data_dir(), Arc::new(secrets::store())));
            network::follow_in_background(app.handle(), paths);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the tauri application")
        .run(|app, event| lifecycle::on_event(app, &event));
}
