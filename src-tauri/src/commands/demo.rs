use std::sync::Arc;

use cornercase_core::demo::{self, DemoTransport};
use cornercase_core::machines::route::Route;
use cornercase_core::transport::Transport;
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Runtime, State};

use crate::commands::connection::{Launcher, Task, launched, start};
use crate::commands::error::CommandError;
use crate::events::{ConnectionChanged, Emit, Emitted};
use crate::state::{Active, AppState, DEMO};
use crate::view::ConnectionView;

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn demo_start<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) {
    enter(&state, app, async_runtime::spawn);
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn demo_leave<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Result<(), CommandError> {
    leave(&state, app, &Launcher::default())
}

pub(crate) fn enter(state: &AppState, emit: impl Emit, spawn: fn(Task) -> JoinHandle<()>) {
    let mut active = state.active();
    if active.as_ref().is_some_and(Active::is_demo) {
        return;
    }
    state.entered_demo(active.as_ref().map(Active::machine));
    *active = Some(launched(state, emit, &launcher(spawn), demo::machine(DEMO)));
}

pub(crate) fn leave(state: &AppState, emit: impl Emit, launcher: &Launcher) -> Result<(), CommandError> {
    let Some(left) = state.left_demo() else {
        return Ok(());
    };
    let mut active = state.active();
    if active.as_ref().is_some_and(Active::is_demo) {
        *active = None;
    }
    drop(active);
    emit.emit(Emitted::Connection(ConnectionChanged { machine: DEMO.0, connection: ConnectionView::None }));
    let known = |id: &_| state.machines(|machines| Ok(machines.machine(*id).is_some())).unwrap_or_default();
    match left.previous.filter(known) {
        Some(id) => start(state, emit, launcher, id),
        None => Ok(()),
    }
}

fn launcher(spawn: fn(Task) -> JoinHandle<()>) -> Launcher {
    let transport = DemoTransport::new();
    Launcher::new(move |_: &Route| Ok(Arc::new(transport.clone()) as Arc<dyn Transport>), spawn)
}

#[cfg(test)]
mod tests;
