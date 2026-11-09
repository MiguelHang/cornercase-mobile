use cornercase_core::cli::keys;
use cornercase_core::cli::last_message::{self, LastMessage};
use cornercase_core::transport::{self, Stdin};
use tauri::State;

use crate::commands::error::CommandError;
use crate::commands::reach::{ask, connected};
use crate::state::AppState;

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn last_message(state: State<'_, AppState>, pane: u64) -> Result<LastMessage, CommandError> {
    read_last_message(&state, pane).await
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn send(state: State<'_, AppState>, pane: u64, text: String) -> Result<(), CommandError> {
    send_message(&state, pane, text).await
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn keys(state: State<'_, AppState>, pane: u64, key: String) -> Result<(), CommandError> {
    press_key(&state, pane, &key).await
}

pub(crate) async fn read_last_message(state: &AppState, pane: u64) -> Result<LastMessage, CommandError> {
    let transport = connected(state)?;
    transport::json(transport.as_ref(), last_message::args(pane)).await.map_err(|error| CommandError::reason(&error))
}

pub(crate) async fn send_message(state: &AppState, pane: u64, text: String) -> Result<(), CommandError> {
    ask(state, last_message::send_args(pane), Some(Stdin::Text(text))).await
}

pub(crate) async fn press_key(state: &AppState, pane: u64, key: &str) -> Result<(), CommandError> {
    let args = keys::args(pane, key).map_err(|error| CommandError::reason(&error))?;
    ask(state, args, None).await
}

#[cfg(test)]
mod tests;
