use cornercase_core::identity::Identity;
use tauri::State;

use crate::commands::error::CommandError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn identity_fingerprint(state: State<'_, AppState>) -> Result<String, CommandError> {
    fingerprint(&state)
}

pub(crate) fn fingerprint(state: &AppState) -> Result<String, CommandError> {
    state.identity().map(Identity::fingerprint)
}

#[cfg(test)]
mod tests;
