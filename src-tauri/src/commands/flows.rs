use cornercase_core::cli::flows::{self, FlowStarted, Flows, StartFlow};
use cornercase_core::transport;
use tauri::State;

use crate::commands::error::CommandError;
use crate::commands::reach::{connected, queue};
use crate::commands::start::answered;
use crate::state::AppState;

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn issue_flows(state: State<'_, AppState>, project: u64) -> Result<Flows, CommandError> {
    let transport = connected(&state)?;
    transport::json(transport.as_ref(), flows::list_args(project)).await.map_err(|error| CommandError::reason(&error))
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn start_flow(state: State<'_, AppState>, form: StartFlow) -> Result<FlowStarted, CommandError> {
    run_flow(&state, form.argv()).await
}

pub(crate) async fn run_flow(state: &AppState, argv: Vec<String>) -> Result<FlowStarted, CommandError> {
    let queue = queue(state)?;
    answered(queue.ask(argv, None).await, flows::flow_started)
}

#[cfg(test)]
mod tests;
