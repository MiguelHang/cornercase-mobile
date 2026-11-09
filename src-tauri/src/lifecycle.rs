use tauri::{AppHandle, Manager, RunEvent, Runtime};

use crate::state::AppState;

pub(crate) fn on_event<R: Runtime>(app: &AppHandle<R>, event: &RunEvent) {
    if foreground(event) {
        resume(app);
    }
}

pub(crate) fn resume<R: Runtime>(app: &AppHandle<R>) {
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
