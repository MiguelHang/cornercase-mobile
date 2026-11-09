use std::pin::Pin;
use std::sync::Arc;

use cornercase_core::cli::status::Snapshot;
use cornercase_core::error::Error;
use cornercase_core::machines::MachineId;
use cornercase_core::machines::pick::{PickConfig, bridge_transports};
use cornercase_core::machines::route::Route;
use cornercase_core::store::Store;
use cornercase_core::transport::Transport;
use cornercase_core::transport::bridge::BridgeConfig;
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Runtime, State};

use crate::commands::error::CommandError;
use crate::events::{Emit, forward};
use crate::state::{Active, AppState};
use crate::view::ConnectionState;

pub(crate) type Task = Pin<Box<dyn Future<Output = ()> + Send>>;
type Transports = dyn Fn(&Route) -> Result<Arc<dyn Transport>, Error> + Send + Sync;

#[derive(Clone)]
pub(crate) struct Launcher {
    transports: Arc<Transports>,
    spawn: fn(Task) -> JoinHandle<()>,
}

impl Launcher {
    pub(crate) fn new(
        transports: impl Fn(&Route) -> Result<Arc<dyn Transport>, Error> + Send + Sync + 'static,
        spawn: fn(Task) -> JoinHandle<()>,
    ) -> Self {
        Self { transports: Arc::new(transports), spawn }
    }

    pub(crate) fn bridge() -> Self {
        Self::new(bridge_transports(BridgeConfig::default()), async_runtime::spawn)
    }
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn connect<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>, id: u64) -> Result<(), CommandError> {
    switch_to(&state, app, &Launcher::bridge(), MachineId(id))
}

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn connection_state(state: State<'_, AppState>) -> ConnectionState {
    current(&state)
}

#[tauri::command]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn session(state: State<'_, AppState>) -> Option<Snapshot> {
    latest_session(&state)
}

pub(crate) fn switch_to(
    state: &AppState,
    emit: impl Emit,
    launcher: &Launcher,
    id: MachineId,
) -> Result<(), CommandError> {
    state.machines(|machines| machines.activate(id).map_err(CommandError::from))?;
    start(state, emit, launcher, id)
}

pub(crate) fn start(state: &AppState, emit: impl Emit, launcher: &Launcher, id: MachineId) -> Result<(), CommandError> {
    let mut active = state.active();
    let machine = state.machines(|machines| machines.machine(id).cloned().ok_or(Error::UnknownMachine.into()))?;
    let transports = Arc::clone(&launcher.transports);
    let (store, driver) = Store::new(machine, move |route: &Route| transports(route), PickConfig::default());
    if state.network_is_down() {
        store.offline();
    }
    let changes = store.changes();
    let forwarder = forward(id, store.clone(), changes, state.shared_machines(), emit);
    let driver = (launcher.spawn)(Box::pin(driver));
    let forwarder = (launcher.spawn)(Box::pin(forwarder));
    *active = Some(Active::new(id, store, driver, forwarder));
    Ok(())
}

pub(crate) fn current(state: &AppState) -> ConnectionState {
    match state.active().as_ref() {
        Some(active) => ConnectionState::of(active.machine(), &active.store().connection()),
        None => ConnectionState::none(),
    }
}

pub(crate) fn latest_session(state: &AppState) -> Option<Snapshot> {
    state.active().as_ref().and_then(|active| active.store().session())
}

#[cfg(test)]
mod tests;
