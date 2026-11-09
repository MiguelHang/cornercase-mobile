#[cfg_attr(not(test), expect(dead_code, reason = "only the tests build the errors until the commands return them"))]
mod commands;
#[cfg_attr(not(test), expect(dead_code, reason = "only the tests forward the changes until a command connects"))]
mod events;
mod identity;
mod secrets;
#[cfg_attr(not(test), expect(dead_code, reason = "only the tests build the state until the app mounts it"))]
mod state;
#[cfg_attr(not(test), expect(dead_code, reason = "only the tests build the views until the commands return them"))]
mod view;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| {
            identity::announce(&secrets::store()).ok();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the tauri application");
}
