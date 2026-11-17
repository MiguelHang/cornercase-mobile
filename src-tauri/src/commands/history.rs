use cornercase_core::error::Error;
use cornercase_core::history::HistoryEntry;
use cornercase_core::history::store::{HistoryStore, JsonFolderStore};
use cornercase_core::machines::MachineId;
use tauri::State;

use crate::commands::error::CommandError;
use crate::state::{AppState, DEMO};

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn history_load(state: State<'_, AppState>, machine: u64, boot: String, pane: u64) -> Vec<HistoryEntry> {
    load(&state, MachineId(machine), &boot, pane).unwrap_or_else(|line| {
        eprintln!("{line}");
        Vec::new()
    })
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn history_save(
    state: State<'_, AppState>,
    machine: u64,
    boot: String,
    pane: u64,
    entries: Vec<HistoryEntry>,
) {
    if let Err(line) = save(&state, MachineId(machine), &boot, pane, &entries) {
        eprintln!("{line}");
    }
}

pub(crate) fn load(state: &AppState, machine: MachineId, boot: &str, pane: u64) -> Result<Vec<HistoryEntry>, String> {
    if machine == DEMO {
        return Ok(state.demo_history(boot, pane));
    }
    held(state, "read", |store| store.load(machine, boot, pane))
}

pub(crate) fn save(
    state: &AppState,
    machine: MachineId,
    boot: &str,
    pane: u64,
    entries: &[HistoryEntry],
) -> Result<(), String> {
    if machine == DEMO {
        state.kept_in_demo(boot, pane, entries);
        return Ok(());
    }
    held(state, "saved", |store| store.save(machine, boot, pane, entries))
}

pub(crate) fn forget(state: &AppState, machine: MachineId) -> Result<(), String> {
    held(state, "removed", |store| store.forget(machine))
}

fn held<T>(state: &AppState, what: &str, work: impl FnOnce(&JsonFolderStore) -> Result<T, Error>) -> Result<T, String> {
    let store = state.history();
    let store = store.as_ref().ok_or_else(|| format!("cornercase-mobile: the history was not {what}: no folder"))?;
    work(store)
        .map_err(|error| format!("cornercase-mobile: the history was not {what}: {:?}", CommandError::from(error).code))
}

#[cfg(test)]
mod tests;
