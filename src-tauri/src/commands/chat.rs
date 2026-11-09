use std::sync::Arc;

use cornercase_core::cli::keys;
use cornercase_core::cli::last_message::{self, LastMessage};
use cornercase_core::error::Error;
use cornercase_core::queue::{Outcome, Queue};
use cornercase_core::transport::{self, Stdin, Transport};
use tauri::State;

use crate::commands::error::CommandError;
use crate::state::AppState;

pub(crate) mod attach;

const ENTER: &str = "enter";

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn last_message(state: State<'_, AppState>, pane: u64) -> Result<LastMessage, CommandError> {
    read_last_message(&state, pane).await
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn send(state: State<'_, AppState>, pane: u64, text: String, force: bool) -> Result<(), CommandError> {
    send_message(&state, pane, text, force).await
}

#[tauri::command(async)]
#[specta::specta]
pub(crate) async fn keys(state: State<'_, AppState>, pane: u64, key: String) -> Result<(), CommandError> {
    press_key(&state, pane, &key).await
}

pub(crate) async fn read_last_message(state: &AppState, pane: u64) -> Result<LastMessage, CommandError> {
    let transport = connected(state)?;
    transport::json(transport.as_ref(), last_message::args(pane)).await.map_err(|error| CommandError::reason(&error))
}

pub(crate) async fn send_message(state: &AppState, pane: u64, text: String, force: bool) -> Result<(), CommandError> {
    match (force, text.is_empty()) {
        (false, _) => ask(state, last_message::send_args(pane), Some(Stdin::Text(text))).await,
        (true, false) => ask(state, last_message::forced_send_args(pane), Some(Stdin::Text(text))).await,
        (true, true) => press_key(state, pane, ENTER).await,
    }
}

pub(crate) async fn press_key(state: &AppState, pane: u64, key: &str) -> Result<(), CommandError> {
    let args = keys::args(pane, key).map_err(|error| CommandError::reason(&error))?;
    ask(state, args, None).await
}

async fn ask(state: &AppState, args: Vec<String>, stdin: Option<Stdin>) -> Result<(), CommandError> {
    asked(state, args, stdin).await.map(drop)
}

async fn asked(state: &AppState, args: Vec<String>, stdin: Option<Stdin>) -> Result<String, CommandError> {
    let queue = queue(state)?;
    answered(queue.ask(args, stdin).await)
}

fn connected(state: &AppState) -> Result<Arc<dyn Transport>, CommandError> {
    state.active().as_ref().and_then(|active| active.store().transport()).ok_or_else(CommandError::not_connected)
}

fn queue(state: &AppState) -> Result<Queue, CommandError> {
    state.active().as_ref().and_then(|active| active.store().queue()).ok_or_else(CommandError::not_connected)
}

fn answered(outcome: Result<Outcome, Error>) -> Result<String, CommandError> {
    match outcome {
        Ok(Outcome::Done(output)) => Ok(output),
        Ok(Outcome::Skipped) => Err(CommandError::not_sent()),
        Err(Error::Unanswered) => Err(CommandError::maybe_sent()),
        Err(error) => Err(CommandError::reason(&error)),
    }
}

#[cfg(test)]
mod tests;
