use cornercase_core::cli::input::InputEvent;
use cornercase_core::error::Error;
use cornercase_core::queue::Queue;
use serde_json::Value;
use tauri::State;

use crate::commands::error::CommandError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn terminal_input(
    state: State<'_, AppState>,
    pane: u64,
    events: Vec<InputEvent>,
) -> Result<(), CommandError> {
    input(&state, pane, &events)
}

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn terminal_scroll(state: State<'_, AppState>, pane: u64, rows_up: i64) -> Result<(), CommandError> {
    scroll(&state, pane, rows_up)
}

pub(crate) fn input(state: &AppState, pane: u64, events: &[InputEvent]) -> Result<(), CommandError> {
    let queue = queue(state)?;
    let events: Vec<Value> =
        events.iter().map(serde_json::to_value).collect::<Result<_, _>>().map_err(|error| unwritten(&error))?;
    for event in events {
        queue.input(pane, event);
    }
    Ok(())
}

pub(crate) fn scroll(state: &AppState, pane: u64, rows_up: i64) -> Result<(), CommandError> {
    queue(state)?.scroll(pane, rows_up);
    Ok(())
}

fn queue(state: &AppState) -> Result<Queue, CommandError> {
    state.active().as_ref().and_then(|active| active.store().queue()).ok_or_else(CommandError::not_connected)
}

fn unwritten(error: &serde_json::Error) -> CommandError {
    CommandError::from(Error::Json(error.to_string()))
}

#[cfg(test)]
mod tests;
