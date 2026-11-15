use cornercase_core::error::Error;
use cornercase_core::machines::MachineId;
use cornercase_core::machines::code::MachineCode;
use tauri::{AppHandle, Runtime, State};

use crate::commands::error::CommandError;
use crate::commands::history;
use crate::events::{ConnectionChanged, Emit, Emitted};
use crate::state::AppState;
use crate::view::{ConnectionView, MachineList};

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn machines_list(state: State<'_, AppState>) -> Result<MachineList, CommandError> {
    list(&state)
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn machine_add(state: State<'_, AppState>, code: String, name: String) -> Result<u64, CommandError> {
    add(&state, &code, &name).map(|id| id.0)
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn machine_remove<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: u64,
) -> Result<(), CommandError> {
    remove(&state, &app, MachineId(id))
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn machine_rename(state: State<'_, AppState>, id: u64, name: String) -> Result<(), CommandError> {
    rename(&state, MachineId(id), &name)
}

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn machine_form_closed(state: State<'_, AppState>) {
    state.forget_redeemed();
}

pub(crate) fn list(state: &AppState) -> Result<MachineList, CommandError> {
    state.machines(|machines| Ok(MachineList::from(&*machines)))
}

pub(crate) fn add(state: &AppState, code: &str, name: &str) -> Result<MachineId, CommandError> {
    let parsed = MachineCode::parse(code)?;
    let secret = state.secret_of(code).ok_or(Error::PairingRefused)?;
    let routes = parsed.cli_routes(&secret);
    let id = state.machines(|machines| machines.add(name, &routes).map_err(CommandError::from))?;
    state.used(code);
    Ok(id)
}

pub(crate) fn rename(state: &AppState, id: MachineId, name: &str) -> Result<(), CommandError> {
    state.machines(|machines| machines.rename(id, name).map_err(CommandError::from))
}

pub(crate) fn remove(state: &AppState, emit: &dyn Emit, id: MachineId) -> Result<(), CommandError> {
    let (removed, gone) = state.machines(|machines| {
        let removed = machines.remove(id).map_err(CommandError::from);
        Ok((removed, machines.machine(id).is_none()))
    })?;
    if gone {
        release(state, emit, id);
    }
    removed
}

fn release(state: &AppState, emit: &dyn Emit, id: MachineId) {
    if let Err(line) = history::forget(state, id) {
        eprintln!("{line}");
    }
    let mut active = state.active();
    if active.as_ref().is_some_and(|active| active.machine() == id) {
        *active = None;
        drop(active);
        emit.emit(Emitted::Connection(ConnectionChanged { machine: id.0, connection: ConnectionView::None }));
    }
}

#[cfg(test)]
mod tests;
