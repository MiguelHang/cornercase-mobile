use std::fs;
use std::io;

use cornercase_core::cli::paste_file::{self, Pasted, Picked};
use cornercase_core::error::Error;
use cornercase_core::transport::Stdin;
use serde::Serialize;
use tauri::async_runtime;
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_photos::Photos;

use crate::commands::error::CommandError;
use crate::commands::reach::{asked, queue};
use crate::state::AppState;

struct Image {
    name: String,
    bytes: Vec<u8>,
    thumbnail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct AttachedImage {
    #[specta(type = u32)]
    pub(crate) id: u64,
    pub(crate) name: String,
    #[specta(type = u32)]
    pub(crate) bytes: u64,
    pub(crate) thumbnail: Option<String>,
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn attach<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    pane: u64,
) -> Result<Option<AttachedImage>, CommandError> {
    attach_image(&state, pane, move || app.state::<Photos<R>>().pick()).await
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn paste_attached(state: State<'_, AppState>, pane: u64, id: u64) -> Result<Pasted, CommandError> {
    paste_image(&state, pane, id).await
}

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn discard(state: State<'_, AppState>, ids: Vec<u64>) {
    state.attached().discard(&ids);
}

pub(crate) async fn attach_image<P>(state: &AppState, pane: u64, pick: P) -> Result<Option<AttachedImage>, CommandError>
where
    P: FnOnce() -> Result<Option<Picked>, Error> + Send + 'static,
{
    queue(state)?;
    let Some(image) = picked(pick).await? else {
        return Ok(None);
    };
    let bytes = u64::try_from(image.bytes.len()).unwrap_or(u64::MAX);
    let id = state.attached().store(pane, image.name.clone(), image.bytes)?;
    Ok(Some(AttachedImage { id, name: image.name, bytes, thumbnail: image.thumbnail }))
}

pub(crate) async fn paste_image(state: &AppState, pane: u64, id: u64) -> Result<Pasted, CommandError> {
    queue(state)?;
    let image = state.attached().get(pane, id).ok_or_else(CommandError::attached_gone)?;
    let output = asked(state, paste_file::args(pane, &image.name), Some(Stdin::Bytes(image.bytes))).await?;
    state.attached().discard(&[id]);
    parsed(&output)
}

fn parsed(output: &str) -> Result<Pasted, CommandError> {
    serde_json::from_str(output).map_err(|error| CommandError::from(Error::Json(error.to_string())))
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
    let read = taken(&picked.path);
    let thumbnail = picked.thumbnail_path.as_deref().map(taken).and_then(Result::ok);
    let bytes = read.map_err(|error| Error::Picker(format!("the photo cannot be read: {error}")))?;
    Ok(Image {
        name: paste_file::output_name(picked.name.as_deref(), picked.kind),
        bytes: paste_file::checked(bytes)?,
        thumbnail: thumbnail.map(|bytes| paste_file::thumbnail_url(&bytes)),
    })
}

fn taken(path: &str) -> io::Result<Vec<u8>> {
    let read = fs::read(path);
    let _ = fs::remove_file(path);
    read
}

#[cfg(test)]
mod tests;
