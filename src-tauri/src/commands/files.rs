use cornercase_core::cli::files::{self, Listing, Read};
use cornercase_core::error::Error;
use serde::Serialize;
use tauri::State;

use crate::commands::error::CommandError;
use crate::commands::reach::connected;
use crate::state::AppState;

const MOST_BYTES: usize = 1 << 20;
const MOST_LINES: usize = 20_000;
const TOO_LONG: &str = "a line of output passed 16 MiB";

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

fn unread(error: &Error) -> CommandError {
    match error {
        Error::Failed { stderr, .. } if stderr.contains(TOO_LONG) => CommandError::too_large_here(),
        _ => CommandError::reason(error),
    }
}

#[cfg(test)]
mod tests;
