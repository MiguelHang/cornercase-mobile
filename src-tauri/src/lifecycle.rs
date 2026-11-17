use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{AppHandle, Manager, RunEvent, Runtime, Webview};

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
    if let Some(active) = state.active().as_ref().filter(|active| !active.is_demo()) {
        active.store().resume_now();
    }
}

pub(crate) fn on_page_load<R: Runtime>(webview: &Webview<R>, payload: &PageLoadPayload<'_>) {
    page_loading(webview, payload.event());
}

pub(crate) fn page_loading<R: Runtime>(manager: &impl Manager<R>, event: PageLoadEvent) {
    if event == PageLoadEvent::Started
        && let Some(state) = manager.try_state::<AppState>()
    {
        state.attached().clear();
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
