use cornercase_core::cli::agents::{self, AgentChoice};
use cornercase_core::cli::start::{self, StartAgent, StartIssue, Started};
use cornercase_core::error::Error;
use cornercase_core::queue::Outcome;
use tauri::State;

use crate::commands::error::CommandError;
use crate::commands::reach::queue;
use crate::state::AppState;
use crate::view::{ProjectChoice, StartChoices};

const ACCOUNT_LIST: [&str; 4] = ["account", "list", "--check", "--json"];

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn start_projects(state: State<'_, AppState>) -> Result<StartChoices, CommandError> {
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
pub(crate) async fn start_issue(state: State<'_, AppState>, form: StartIssue) -> Result<Started, CommandError> {
    run(&state, form.argv()).await
}

pub(crate) fn projects(state: &AppState) -> Result<StartChoices, CommandError> {
    let active = state.active();
    let store = active.as_ref().ok_or_else(CommandError::not_connected)?.store();
    let session = store.session().unwrap_or_default();
    Ok(StartChoices { boot: store.boot(), projects: session.projects.iter().map(ProjectChoice::from).collect() })
}

pub(crate) async fn installed(state: &AppState) -> Result<Vec<AgentChoice>, CommandError> {
    let queue = queue(state)?;
    answered(queue.ask(ACCOUNT_LIST.map(str::to_owned).to_vec(), None).await, agents::installed)
}

pub(crate) async fn run(state: &AppState, argv: Vec<String>) -> Result<Started, CommandError> {
    let queue = queue(state)?;
    answered(queue.ask(argv, None).await, start::started)
}

pub(crate) fn answered<T>(
    outcome: Result<Outcome, Error>,
    read: fn(&str) -> Result<T, Error>,
) -> Result<T, CommandError> {
    match outcome {
        Ok(Outcome::Done(text)) => Ok(read(&text)?),
        Ok(Outcome::Skipped) => Err(CommandError::not_sent()),
        Err(error) => Err(CommandError::reason(&error)),
    }
}

#[cfg(test)]
mod tests;
