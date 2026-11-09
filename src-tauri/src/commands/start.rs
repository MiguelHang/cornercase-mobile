use cornercase_core::cli::agents::{self, AgentChoice};
use cornercase_core::cli::start::{self, NewWorkspace, StartAgent, Started};
use cornercase_core::error::Error;
use cornercase_core::queue::Outcome;
use tauri::State;

use crate::commands::error::{CommandError, ErrorCode};
use crate::state::AppState;
use crate::view::ProjectChoice;

const ACCOUNT_LIST: [&str; 4] = ["account", "list", "--check", "--json"];

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn start_projects(state: State<'_, AppState>) -> Result<Vec<ProjectChoice>, CommandError> {
    projects(&state)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn installed_agents(state: State<'_, AppState>) -> Result<Vec<AgentChoice>, CommandError> {
    installed(&state).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn start_agent(state: State<'_, AppState>, form: StartAgent) -> Result<Started, CommandError> {
    run(&state, form.argv()).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn new_workspace(state: State<'_, AppState>, form: NewWorkspace) -> Result<Started, CommandError> {
    run(&state, form.argv()).await
}

pub(crate) fn projects(state: &AppState) -> Result<Vec<ProjectChoice>, CommandError> {
    let active = state.active();
    let active = active.as_ref().ok_or_else(CommandError::not_connected)?;
    let session = active.store().session().unwrap_or_default();
    Ok(session.projects.iter().map(ProjectChoice::from).collect())
}

pub(crate) async fn installed(state: &AppState) -> Result<Vec<AgentChoice>, CommandError> {
    let queue = state.active().as_ref().and_then(|active| active.store().queue());
    let queue = queue.ok_or_else(CommandError::not_connected)?;
    match queue.ask(ACCOUNT_LIST.map(str::to_owned).to_vec(), None).await? {
        Outcome::Done(text) => Ok(agents::installed(&text)?),
        Outcome::Skipped => Err(Error::Unanswered.into()),
    }
}

pub(crate) async fn run(state: &AppState, argv: Vec<String>) -> Result<Started, CommandError> {
    let queue = state.active().as_ref().and_then(|active| active.store().queue());
    let queue = queue.ok_or_else(CommandError::not_connected)?;
    match queue.ask(argv, None).await.map_err(refused)? {
        Outcome::Done(text) => Ok(start::started(&text)?),
        Outcome::Skipped => Err(Error::Unanswered.into()),
    }
}

fn refused(error: Error) -> CommandError {
    let Error::Failed { stderr, .. } = &error else { return error.into() };
    let stderr = stderr.trim();
    let message = stderr.strip_prefix("Error: ").unwrap_or(stderr).trim();
    if message.is_empty() {
        return error.into();
    }
    CommandError { code: ErrorCode::Failed, message: message.to_owned() }
}

#[cfg(test)]
mod tests;
