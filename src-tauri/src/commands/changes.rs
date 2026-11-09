use cornercase_core::changes::{Changes, Seen};
use cornercase_core::cli::changes::{self, Mark, Mode, Patch};
use cornercase_core::transport;
use serde::Deserialize;
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Runtime, State};

use crate::commands::connection::Task;
use crate::commands::error::CommandError;
use crate::commands::reach::{ask, connected};
use crate::events::{ChangesChanged, Emit, Emitted, warned};
use crate::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
pub(crate) struct ViewedFile {
    #[specta(type = u32)]
    pub(crate) workspace: u64,
    pub(crate) path: String,
    pub(crate) viewed: bool,
    pub(crate) digest: Option<String>,
    pub(crate) mode: Mode,
}

impl ViewedFile {
    fn argv(&self) -> Result<Vec<String>, CommandError> {
        let mark = match (self.viewed, self.digest.as_deref().filter(|digest| !digest.is_empty())) {
            (true, Some(digest)) => Mark::Seen { digest: digest.to_owned() },
            (true, None) => return Err(CommandError::no_digest()),
            (false, _) => Mark::Clear { mode: self.mode },
        };
        Ok(changes::viewed_args(self.workspace, &self.path, &mark))
    }
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn changes_file(
    state: State<'_, AppState>,
    workspace: u64,
    path: String,
    mode: Mode,
) -> Result<Patch, CommandError> {
    read_file(&state, workspace, &path, mode).await
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn changes_viewed(state: State<'_, AppState>, file: ViewedFile) -> Result<(), CommandError> {
    mark_viewed(&state, &file).await
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn changes_follow<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    workspace: u64,
    mode: Mode,
) -> Result<(), CommandError> {
    follow(&state, app, async_runtime::spawn, Changes { workspace, mode })
}

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn changes_unfollow(state: State<'_, AppState>, workspace: u64) {
    unfollow(&state, workspace);
}

pub(crate) fn follow(
    state: &AppState,
    emit: impl Emit,
    spawn: fn(Task) -> JoinHandle<()>,
    changes: Changes,
) -> Result<(), CommandError> {
    let mut active = state.active();
    let active = active.as_mut().ok_or_else(CommandError::not_connected)?;
    let machine = active.machine().0;
    let running = changes.run(active.store().clone(), move |seen| emit.emit(told(machine, changes, seen)));
    active.follow(changes.workspace, spawn(Box::pin(running)));
    Ok(())
}

pub(crate) fn unfollow(state: &AppState, workspace: u64) {
    if let Some(active) = state.active().as_mut() {
        active.unfollow(workspace);
    }
}

fn told(machine: u64, changes: Changes, seen: Seen) -> Emitted {
    let Changes { workspace, mode } = changes;
    let listed = |summary, closed| Emitted::Changes(ChangesChanged { machine, workspace, mode, summary, closed });
    match seen {
        Seen::Summary(summary) => listed(Some(summary), false),
        Seen::Failed(error) => warned(&error),
        Seen::Closed => listed(None, true),
    }
}

pub(crate) async fn read_file(state: &AppState, workspace: u64, path: &str, mode: Mode) -> Result<Patch, CommandError> {
    let transport = connected(state)?;
    let argv = changes::file_args(workspace, path, mode);
    transport::json(transport.as_ref(), argv).await.map_err(|error| CommandError::reason(&error))
}

pub(crate) async fn mark_viewed(state: &AppState, file: &ViewedFile) -> Result<(), CommandError> {
    ask(state, file.argv()?, None).await
}

#[cfg(test)]
mod tests;
