use cornercase_core::cli::surface::{Colours, Size};
use cornercase_core::machines::MachineId;
use cornercase_core::store::Store;
use cornercase_core::terminal::asking::{Asked, Asking, Question};
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Runtime, State};
use tokio::sync::mpsc;

use crate::commands::connection::Task;
use crate::commands::error::CommandError;
use crate::commands::terminal::{Batching, by_frame};
use crate::events::{Emit, Emitted, QuestionChanged};
use crate::state::AppState;

const SIZE: Size = Size { cols: 80, rows: 24 };

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn question_open<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    tab: u64,
    pane: u64,
) -> Result<(), CommandError> {
    open(&state, app, async_runtime::spawn, Asking { tab, pane, size: SIZE })
}

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn question_close(state: State<'_, AppState>, tab: u64) {
    close(&state, tab);
}

pub(crate) fn open(
    state: &AppState,
    emit: impl Emit,
    spawn: fn(Task) -> JoinHandle<()>,
    asking: Asking,
) -> Result<(), CommandError> {
    let mut active = state.active();
    let active = active.as_mut().ok_or_else(CommandError::not_connected)?;
    let asked = ask(active.machine(), asking, active.store().clone(), emit);
    active.ask(asking.tab, spawn(Box::pin(asked)));
    Ok(())
}

pub(crate) fn close(state: &AppState, tab: u64) {
    if let Some(active) = state.active().as_mut() {
        active.unask(tab);
    }
}

async fn ask(machine: MachineId, asking: Asking, store: Store, emit: impl Emit) {
    let (said, heard) = mpsc::unbounded_channel();
    let running = asking.run(store, move |asked| {
        said.send(asked).ok();
    });
    tokio::join!(running, by_frame(Asks::new(machine, asking), heard, emit));
}

pub(crate) struct Asks {
    machine: u64,
    tab: u64,
    pane: u64,
    pending: Option<QuestionChanged>,
}

impl Asks {
    pub(crate) fn new(machine: MachineId, asking: Asking) -> Self {
        Self { machine: machine.0, tab: asking.tab, pane: asking.pane, pending: None }
    }

    fn changed(&self, question: Option<Question>, closed: bool) -> QuestionChanged {
        let Question { pane, colours, rows, options } = question.unwrap_or_else(|| self.nothing());
        QuestionChanged { machine: self.machine, tab: self.tab, pane, colours, rows, options, closed }
    }

    fn nothing(&self) -> Question {
        Question { pane: self.pane, colours: Colours::default(), rows: Vec::new(), options: Vec::new() }
    }
}

impl Batching for Asks {
    type Heard = Asked;

    fn add(&mut self, asked: Asked) {
        self.pending = Some(match asked {
            Asked::Question(question) => self.changed(Some(question), false),
            Asked::Failed(_) => self.changed(None, false),
            Asked::Closed => self.changed(None, true),
        });
    }

    fn take(&mut self) -> Vec<Emitted> {
        self.pending.take().map(Emitted::Question).into_iter().collect()
    }
}

#[cfg(test)]
mod tests;
