use cornercase_core::error::Error;
use cornercase_core::machines::MachineId;
use cornercase_core::machines::code::MachineCode;
use cornercase_core::machines::registry::{Direction, Machines};
use cornercase_core::machines::route::{Address, Route};
use cornercase_core::store::Connection;
use tauri::{AppHandle, Runtime, State};

use crate::commands::connection::{Launcher, start};
use crate::commands::error::CommandError;
use crate::events::Emit;
use crate::state::AppState;
use crate::view::MergeView;

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn machine_route_add<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: u64,
    code: String,
) -> Result<(), CommandError> {
    add_route(&state, app, &Launcher::bridge(), MachineId(id), &code)
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn machine_merge<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: u64,
    code: String,
) -> Result<MergeView, CommandError> {
    merge(&state, app, &Launcher::bridge(), MachineId(id), &code)
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn machine_route_remove<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: u64,
    url: String,
) -> Result<(), CommandError> {
    remove_route(&state, app, &Launcher::bridge(), MachineId(id), &url)
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn machine_route_move(
    state: State<'_, AppState>,
    id: u64,
    url: String,
    direction: Direction,
) -> Result<(), CommandError> {
    move_route(&state, MachineId(id), &url, direction)
}

pub(crate) fn add_route(
    state: &AppState,
    emit: impl Emit,
    launcher: &Launcher,
    id: MachineId,
    code: &str,
) -> Result<(), CommandError> {
    let routes = MachineCode::parse(code)?.routes();
    state.machines(|machines| {
        routes.into_iter().try_for_each(|route| machines.add_route(id, route)).map_err(CommandError::from)
    })?;
    if connected_to(state, id).is_some() {
        start(state, emit, launcher, id)?;
    }
    Ok(())
}

pub(crate) fn merge(
    state: &AppState,
    emit: impl Emit,
    launcher: &Launcher,
    id: MachineId,
    code: &str,
) -> Result<MergeView, CommandError> {
    let code = MachineCode::parse(code)?;
    let merged = state.machines(|machines| machines.merge(id, &code).map_err(CommandError::from))?;
    let changed = merged.token_updated || !merged.added.is_empty();
    if changed && connected_to(state, id).is_some() {
        start(state, emit, launcher, id)?;
    }
    Ok(MergeView::from(&merged))
}

pub(crate) fn remove_route(
    state: &AppState,
    emit: impl Emit,
    launcher: &Launcher,
    id: MachineId,
    url: &str,
) -> Result<(), CommandError> {
    let address = state.machines(|machines| address_of(machines, id, url).map_err(CommandError::from))?;
    let seen = connected_to(state, id).is_some_and(|connection| races_through(&connection, &address));
    state.machines(|machines| machines.remove_route(id, &address).map_err(CommandError::from))?;
    if seen {
        start(state, emit, launcher, id)?;
    }
    Ok(())
}

pub(crate) fn move_route(state: &AppState, id: MachineId, url: &str, direction: Direction) -> Result<(), CommandError> {
    state.machines(|machines| {
        let address = address_of(machines, id, url)?;
        machines.move_route(id, &address, direction).map_err(CommandError::from)
    })
}

fn address_of(machines: &Machines, id: MachineId, url: &str) -> Result<Address, Error> {
    let machine = machines.machine(id).ok_or(Error::UnknownMachine)?;
    machine
        .routes
        .iter()
        .map(Route::address)
        .find(|address| address.to_string() == url)
        .cloned()
        .ok_or(Error::UnknownRoute)
}

fn connected_to(state: &AppState, id: MachineId) -> Option<Connection> {
    state.active().as_ref().filter(|active| active.machine() == id).map(|active| active.store().connection())
}

fn races_through(connection: &Connection, address: &Address) -> bool {
    match connection {
        Connection::Connected { route } => route == address,
        _ => true,
    }
}

#[cfg(test)]
mod tests;
