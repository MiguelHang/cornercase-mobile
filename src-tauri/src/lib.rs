use std::sync::Arc;

use tauri::Manager;

use crate::state::AppState;

mod bindings;
mod commands;
mod events;
mod identity;
mod lifecycle;
mod secrets;
mod state;
#[cfg(test)]
mod test_util;
mod view;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta = bindings::specta();
    tauri::Builder::default()
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            specta.mount_events(app);
            let state = AppState::load(app.path().app_data_dir(), Arc::new(secrets::store()));
            lifecycle::started(&state, app.handle());
            app.manage(state);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the tauri application")
        .run(|app, event| lifecycle::on_event(app, &event));
}
