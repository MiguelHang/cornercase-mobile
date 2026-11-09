use cornercase_core::cli::changes::{self, Mark, Mode, Patch};
use cornercase_core::transport;
use serde::Deserialize;
use tauri::State;

use crate::commands::error::CommandError;
use crate::commands::reach::{ask, connected};
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
