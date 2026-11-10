use cornercase_core::cli::files::{self, Found, Listing, Read};
use cornercase_core::error::Error;
use cornercase_core::protocol::LINE_TOO_LONG;
use serde::Serialize;
use tauri::State;
use tauri::async_runtime::{self, JoinHandle};
use tokio::sync::oneshot::{self, Receiver};

use crate::commands::connection::Task;
use crate::commands::error::CommandError;
use crate::commands::reach::connected;
use crate::state::AppState;

const MOST_BYTES: usize = 1 << 20;
const MOST_LINES: usize = 20_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "shown", rename_all = "snake_case")]
pub(crate) enum FileShown {
    Read {
        read: Read,
    },
    TooLargeHere {
        #[specta(type = u32)]
        bytes: usize,
        #[specta(type = u32)]
        lines: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "searched", rename_all = "snake_case")]
pub(crate) enum Searched {
    Found { found: Found },
    Cancelled,
}

impl FileShown {
    fn of(read: Read) -> Self {
        match read.text.as_deref().map(|text| (text.len(), text.lines().count())) {
            Some((bytes, lines)) if bytes > MOST_BYTES || lines > MOST_LINES => Self::TooLargeHere { bytes, lines },
            _ => Self::Read { read },
        }
    }
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn files_list(
    state: State<'_, AppState>,
    workspace: u64,
    paths: Vec<String>,
) -> Result<Listing, CommandError> {
    let transport = connected(&state)?;
    let argv = files::list_args(workspace, &paths);
    files::partial_json(transport.as_ref(), argv).await.map_err(|error| CommandError::reason(&error))
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn files_read(
    state: State<'_, AppState>,
    workspace: u64,
    path: String,
) -> Result<FileShown, CommandError> {
    let transport = connected(&state)?;
    let argv = files::read_args(workspace, &path);
    let read = files::partial_json(transport.as_ref(), argv).await.map_err(|error| unread(&error))?;
    Ok(FileShown::of(read))
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn files_search(
    state: State<'_, AppState>,
    workspace: u64,
    query: String,
) -> Result<Searched, CommandError> {
    search(&state, async_runtime::spawn, workspace, &query).await
}

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn files_search_stop(state: State<'_, AppState>, workspace: u64) {
    stop_search(&state, workspace);
}

async fn search(
    state: &AppState,
    spawn: fn(Task) -> JoinHandle<()>,
    workspace: u64,
    query: &str,
) -> Result<Searched, CommandError> {
    let searching = started(state, spawn, workspace, files::search_args(workspace, query))?;
    match searching.await {
        Ok(found) => found.map(|found| Searched::Found { found }).map_err(|error| CommandError::reason(&error)),
        Err(_) => Ok(Searched::Cancelled),
    }
}

fn started(
    state: &AppState,
    spawn: fn(Task) -> JoinHandle<()>,
    workspace: u64,
    argv: Vec<String>,
) -> Result<Receiver<Result<Found, Error>>, CommandError> {
    let mut active = state.active();
    let active = active.as_mut().ok_or_else(CommandError::not_connected)?;
    let transport = active.store().transport().ok_or_else(CommandError::not_connected)?;
    let (sender, found) = oneshot::channel();
    let searching = async move {
        sender.send(files::partial_json(transport.as_ref(), argv).await).ok();
    };
    active.search(workspace, spawn(Box::pin(searching)));
    Ok(found)
}

fn stop_search(state: &AppState, workspace: u64) {
    if let Some(active) = state.active().as_mut() {
        active.unsearch(workspace);
    }
}

fn unread(error: &Error) -> CommandError {
    match error {
        Error::Failed { stderr, .. } if stderr.contains(LINE_TOO_LONG) => CommandError::too_large_here(),
        _ => CommandError::reason(error),
    }
}

#[cfg(test)]
mod tests;
