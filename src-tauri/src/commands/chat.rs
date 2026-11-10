use cornercase_core::cli::last_message::{self, LastMessage};
use cornercase_core::cli::{keys, mark};
use cornercase_core::transport::{self, Stdin};
use tauri::State;

use crate::commands::error::CommandError;
use crate::commands::reach::{ask, connected};
use crate::state::AppState;

pub(crate) mod attach;
pub(crate) mod question;

const ENTER: &str = "enter";

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn last_message(state: State<'_, AppState>, pane: u64) -> Result<LastMessage, CommandError> {
    read_last_message(&state, pane).await
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn send(state: State<'_, AppState>, pane: u64, text: String, force: bool) -> Result<(), CommandError> {
    send_message(&state, pane, text, force).await
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn keys(state: State<'_, AppState>, pane: u64, keys: Vec<String>) -> Result<(), CommandError> {
    let names: Vec<&str> = keys.iter().map(String::as_str).collect();
    press_key(&state, pane, &names).await
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn mark_reviewed(state: State<'_, AppState>, pane: u64) -> Result<(), CommandError> {
    let marked = mark_pane_reviewed(&state, pane).await;
    if let Err(error) = &marked {
        eprintln!("cornercase-mobile: mark --pane {pane} --reviewed failed: {}", error.message);
    }
    marked
}

pub(crate) async fn read_last_message(state: &AppState, pane: u64) -> Result<LastMessage, CommandError> {
    let transport = connected(state)?;
    transport::json(transport.as_ref(), last_message::args(pane)).await.map_err(|error| CommandError::reason(&error))
}

pub(crate) async fn send_message(state: &AppState, pane: u64, text: String, force: bool) -> Result<(), CommandError> {
    match (force, text.is_empty()) {
        (false, _) => ask(state, last_message::send_args(pane), Some(Stdin::Text(text))).await,
        (true, false) => ask(state, last_message::forced_send_args(pane), Some(Stdin::Text(text))).await,
        (true, true) => press_key(state, pane, &[ENTER]).await,
    }
}

pub(crate) async fn press_key(state: &AppState, pane: u64, names: &[&str]) -> Result<(), CommandError> {
    let args = keys::args(pane, names).map_err(|error| CommandError::reason(&error))?;
    ask(state, args, None).await
}

pub(crate) async fn mark_pane_reviewed(state: &AppState, pane: u64) -> Result<(), CommandError> {
    ask(state, mark::reviewed_args(pane), None).await
}

#[cfg(test)]
mod tests;
