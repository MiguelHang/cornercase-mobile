use std::fs;

use cornercase_core::cli::paste_file::{self, Pasted, Picked};
use cornercase_core::error::Error;
use cornercase_core::transport::Stdin;
use tauri::async_runtime;
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_photos::Photos;

use crate::commands::error::CommandError;
use crate::commands::reach::{asked, queue};
use crate::state::AppState;

struct Image {
    name: String,
    bytes: Vec<u8>,
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn attach<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    pane: u64,
) -> Result<Option<Pasted>, CommandError> {
    attach_image(&state, pane, move || app.state::<Photos<R>>().pick()).await
}

pub(crate) async fn attach_image<P>(state: &AppState, pane: u64, pick: P) -> Result<Option<Pasted>, CommandError>
where
    P: FnOnce() -> Result<Option<Picked>, Error> + Send + 'static,
{
    queue(state)?;
    let Some(image) = picked(pick).await? else {
        return Ok(None);
    };
    let output = asked(state, paste_file::args(pane, &image.name), Some(Stdin::Bytes(image.bytes))).await?;
    serde_json::from_str(&output).map(Some).map_err(|error| CommandError::from(Error::Json(error.to_string())))
}

async fn picked<P>(pick: P) -> Result<Option<Image>, CommandError>
where
    P: FnOnce() -> Result<Option<Picked>, Error> + Send + 'static,
{
    let read = async_runtime::spawn_blocking(move || pick()?.as_ref().map(read).transpose())
        .await
        .map_err(|error| Error::Picker(error.to_string()))?;
    Ok(read?)
}

fn read(picked: &Picked) -> Result<Image, Error> {
    let read = fs::read(&picked.path);
    let _ = fs::remove_file(&picked.path);
    let bytes = read.map_err(|error| Error::Picker(format!("the photo cannot be read: {error}")))?;
    Ok(Image { name: paste_file::output_name(picked.name.as_deref(), picked.kind), bytes: paste_file::checked(bytes)? })
}

#[cfg(test)]
mod tests;
