use std::collections::BTreeMap;
use std::time::Duration;

use cornercase_core::cli::surface::{Colours, Modes, Size};
use cornercase_core::machines::MachineId;
use cornercase_core::store::Store;
use cornercase_core::terminal::live::{Screen, Terminal, Unfitted};
use cornercase_core::terminal::shown::Update;
use serde::Deserialize;
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Runtime, State};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::time;

use crate::commands::connection::Task;
use crate::commands::error::CommandError;
use crate::events::{Emit, Emitted, TerminalChanged, Toast, warned};
use crate::state::AppState;

pub(crate) mod input;

const FRAME: Duration = Duration::from_millis(16);
const TAKEN_BACK: &str = "The machine took this tab back";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, specta::Type)]
pub(crate) struct Shape {
    pub(crate) size: Size,
    pub(crate) fit: bool,
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn terminal_open<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    tab: u64,
    pane: Option<u64>,
    shape: Shape,
) -> Result<(), CommandError> {
    open(&state, app, async_runtime::spawn, Terminal { tab, pane, size: shape.size, fit: shape.fit })
}

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn terminal_close(state: State<'_, AppState>, tab: u64) {
    close(&state, tab);
}

pub(crate) fn open(
    state: &AppState,
    emit: impl Emit,
    spawn: fn(Task) -> JoinHandle<()>,
    terminal: Terminal,
) -> Result<(), CommandError> {
    let mut active = state.active();
    let active = active.as_mut().ok_or_else(CommandError::not_connected)?;
    let shown = show(active.machine(), terminal, active.store().clone(), emit);
    active.show(terminal.tab, spawn(Box::pin(shown)));
    Ok(())
}

pub(crate) fn close(state: &AppState, tab: u64) {
    if let Some(active) = state.active().as_mut() {
        active.hide(tab);
    }
}

async fn show(machine: MachineId, terminal: Terminal, store: Store, emit: impl Emit) {
    let (screens, heard) = mpsc::unbounded_channel();
    let running = terminal.run(store, move |screen| {
        screens.send(screen).ok();
    });
    tokio::join!(running, by_frame(Batch::new(machine, terminal.tab), heard, emit));
}

pub(crate) trait Batching {
    type Heard;

    fn add(&mut self, heard: Self::Heard);

    fn take(&mut self) -> Vec<Emitted>;
}

pub(crate) async fn by_frame<B: Batching>(mut batch: B, mut heard: UnboundedReceiver<B::Heard>, emit: impl Emit) {
    while let Some(screen) = heard.recv().await {
        batch.add(screen);
        while let Ok(screen) = heard.try_recv() {
            batch.add(screen);
        }
        for event in batch.take() {
            emit.emit(event);
        }
        time::sleep(FRAME).await;
    }
}

pub(crate) struct Batch {
    machine: MachineId,
    tab: u64,
    pending: Option<TerminalChanged>,
    told: Vec<Emitted>,
}

impl Batching for Batch {
    type Heard = Screen;

    fn add(&mut self, screen: Screen) {
        match screen {
            Screen::Update(update) => self.join(update),
            Screen::Failed(error) => self.told.push(warned(&error)),
            Screen::Closed => self.held().closed = true,
            Screen::Fit(unfitted) => self.unfit(unfitted),
        }
    }

    fn take(&mut self) -> Vec<Emitted> {
        let changed = self.pending.take().map(Emitted::Terminal);
        changed.into_iter().chain(self.told.drain(..)).collect()
    }
}

impl Batch {
    pub(crate) fn new(machine: MachineId, tab: u64) -> Self {
        Self { machine, tab, pending: None, told: Vec::new() }
    }

    fn join(&mut self, update: Update) {
        let held = self.held();
        if update.replaced {
            held.rows.clear();
            held.replaced = true;
        }
        held.rows.extend(update.rows);
        held.pane = Some(update.pane);
        held.size = update.size;
        held.cursor = update.cursor;
        held.colours = update.colours;
        held.modes = update.modes;
    }

    fn unfit(&mut self, unfitted: Unfitted) {
        self.held().unfitted = true;
        self.told.push(match unfitted {
            Unfitted::TakenBack => Emitted::Toast(Toast::info(TAKEN_BACK)),
            Unfitted::Refused(error) => warned(&error),
        });
    }

    fn held(&mut self) -> &mut TerminalChanged {
        let (machine, tab) = (self.machine.0, self.tab);
        self.pending.get_or_insert_with(|| TerminalChanged {
            machine,
            tab,
            pane: None,
            size: Size::default(),
            cursor: None,
            colours: Colours::default(),
            modes: Modes::default(),
            replaced: false,
            rows: BTreeMap::new(),
            closed: false,
            unfitted: false,
        })
    }
}

#[cfg(test)]
mod tests;
