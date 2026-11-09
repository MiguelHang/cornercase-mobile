use tauri::{AppHandle, Manager, RunEvent, Runtime};

use crate::events::{Emit, Emitted, Toast};
use crate::state::AppState;

pub(crate) fn started(state: &AppState, emit: &impl Emit) {
    if let Err(error) = state.machines(|_| Ok(())) {
        emit.emit(Emitted::Toast(Toast::error(error.message)));
    }
}

pub(crate) fn on_event<R: Runtime>(app: &AppHandle<R>, event: &RunEvent) {
    if !foreground(event) {
        return;
    }
    if let Some(state) = app.try_state::<AppState>() {
        on_foreground(&state);
    }
}

pub(crate) fn on_foreground(state: &AppState) {
    if let Some(active) = state.active().as_ref() {
        active.store().resume_now();
    }
}

#[cfg(mobile)]
fn foreground(event: &RunEvent) -> bool {
    matches!(event, RunEvent::WindowEvent { event: tauri::WindowEvent::Resumed, .. })
}

#[cfg(not(mobile))]
fn foreground(_: &RunEvent) -> bool {
    false
}

#[cfg(test)]
mod tests;
