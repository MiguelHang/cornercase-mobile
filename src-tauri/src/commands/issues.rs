use cornercase_core::cli::issues::{self, ListFilter, Lists, ReadIssue, Sources};
use cornercase_core::transport::{self, partial_json};
use tauri::State;

use crate::commands::error::CommandError;
use crate::commands::reach::connected;
use crate::state::AppState;

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn issue_sources(state: State<'_, AppState>, project: u64) -> Result<Sources, CommandError> {
    let transport = connected(&state)?;
    transport::json(transport.as_ref(), issues::sources_args(project))
        .await
        .map_err(|error| CommandError::reason(&error))
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn issue_list(
    state: State<'_, AppState>,
    project: u64,
    filter: ListFilter,
) -> Result<Lists, CommandError> {
    let transport = connected(&state)?;
    let argv = issues::list_args(project, &filter);
    partial_json(transport.as_ref(), argv).await.map_err(|error| CommandError::reason(&error))
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn issue_read(
    state: State<'_, AppState>,
    project: u64,
    source: String,
    key: String,
) -> Result<ReadIssue, CommandError> {
    let transport = connected(&state)?;
    let argv = issues::read_args(project, &source, &key);
    transport::json(transport.as_ref(), argv).await.map_err(|error| CommandError::reason(&error))
}

#[cfg(test)]
mod tests;
