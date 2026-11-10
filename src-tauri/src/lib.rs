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
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_network::init(hear))
        .plugin(tauri_plugin_photos::init())
        .plugin(tauri_plugin_secrets::init());
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_barcode_scanner::init()).plugin(tauri_plugin_haptics::init());
    builder
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            specta.mount_events(app);
            let data_dir = app.path().app_data_dir();
            let store = secrets::store(app.handle());
            let routes = secrets::routes(data_dir.as_deref().ok(), Arc::clone(&store));
            app.manage(AppState::load(data_dir, &*store, routes));
            network::follow_in_background(app.handle(), paths);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the tauri application")
        .run(|app, event| lifecycle::on_event(app, &event));
}
