use std::sync::Arc;

use cornercase_core::cli::last_message::{self, LastMessage};
use cornercase_core::transport::{self, Transport};
use tauri::State;

use crate::commands::error::CommandError;
use crate::state::AppState;

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn last_message(state: State<'_, AppState>, pane: u64) -> Result<LastMessage, CommandError> {
    read_last_message(&state, pane).await
}

pub(crate) async fn read_last_message(state: &AppState, pane: u64) -> Result<LastMessage, CommandError> {
    let transport = connected(state)?;
    transport::json(transport.as_ref(), last_message::args(pane)).await.map_err(|error| CommandError::reason(&error))
}

fn connected(state: &AppState) -> Result<Arc<dyn Transport>, CommandError> {
    state.active().as_ref().and_then(|active| active.store().transport()).ok_or_else(CommandError::not_connected)
}

#[cfg(test)]
mod tests;
