use cornercase_core::cli::close::{self, AgentClosing};
use tauri::State;

use crate::commands::error::CommandError;
use crate::commands::reach::{asked, json};
use crate::state::AppState;

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn close_agent_check(
    state: State<'_, AppState>,
    pane: u64,
    boot: String,
) -> Result<AgentClosing, CommandError> {
    closing(&state, pane, &boot, true).await
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn close_agent(
    state: State<'_, AppState>,
    pane: u64,
    boot: String,
) -> Result<AgentClosing, CommandError> {
    closing(&state, pane, &boot, false).await
}

pub(crate) async fn closing(
    state: &AppState,
    pane: u64,
    boot: &str,
    check: bool,
) -> Result<AgentClosing, CommandError> {
    let output = asked(state, close::agent_args(pane, boot, check), None).await?;
    json(&output)
}

#[cfg(test)]
mod tests;
