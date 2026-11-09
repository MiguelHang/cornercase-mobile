use std::sync::Arc;

use cornercase_core::error::Error;
use cornercase_core::queue::{Outcome, Queue};
use cornercase_core::transport::{Stdin, Transport};

use crate::commands::error::CommandError;
use crate::state::AppState;

pub(crate) fn connected(state: &AppState) -> Result<Arc<dyn Transport>, CommandError> {
    state.active().as_ref().and_then(|active| active.store().transport()).ok_or_else(CommandError::not_connected)
}

pub(crate) fn queue(state: &AppState) -> Result<Queue, CommandError> {
    state.active().as_ref().and_then(|active| active.store().queue()).ok_or_else(CommandError::not_connected)
}

pub(crate) async fn ask(state: &AppState, args: Vec<String>, stdin: Option<Stdin>) -> Result<(), CommandError> {
    asked(state, args, stdin).await.map(drop)
}

pub(crate) async fn asked(state: &AppState, args: Vec<String>, stdin: Option<Stdin>) -> Result<String, CommandError> {
    let queue = queue(state)?;
    answered(queue.ask(args, stdin).await)
}

pub(crate) fn answered(outcome: Result<Outcome, Error>) -> Result<String, CommandError> {
    match outcome {
        Ok(Outcome::Done(output)) => Ok(output),
        Ok(Outcome::Skipped) => Err(CommandError::not_sent()),
        Err(Error::Unanswered) => Err(CommandError::maybe_sent()),
        Err(error) => Err(CommandError::reason(&error)),
    }
}

#[cfg(test)]
mod tests;
