use std::env;
use std::pin::Pin;
use std::sync::Arc;

use cornercase_core::board::AgentRow;
use cornercase_core::error::Error;
use cornercase_core::machines::code::MachineCode;
use cornercase_core::machines::pairing::{PairConfig, PairError, pair};
use cornercase_core::machines::pick::{PickConfig, websocket_transports};
use cornercase_core::machines::route::{Route, Token};
use cornercase_core::machines::{Machine, MachineId};
use cornercase_core::store::Store;
use cornercase_core::transport::Transport;
use cornercase_core::transport::websocket::SocketConfig;
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Runtime, State};

use crate::commands::error::CommandError;
use crate::events::{Emit, forward};
use crate::state::{Active, AppState, DEMO};
use crate::view::ConnectionState;

pub(crate) type Task = Pin<Box<dyn Future<Output = ()> + Send>>;
pub(crate) type Pairing = Pin<Box<dyn Future<Output = Result<Token, PairError>> + Send>>;
type Transports = dyn Fn(&Route) -> Result<Arc<dyn Transport>, Error> + Send + Sync;
type Pairer = dyn Fn(MachineCode) -> Pairing + Send + Sync;

#[derive(Clone)]
pub(crate) struct Launcher {
    transports: Arc<Transports>,
    pairer: Arc<Pairer>,
    spawn: fn(Task) -> JoinHandle<()>,
}

impl Launcher {
    pub(crate) fn new(
        transports: impl Fn(&Route) -> Result<Arc<dyn Transport>, Error> + Send + Sync + 'static,
        spawn: fn(Task) -> JoinHandle<()>,
    ) -> Self {
        Self { transports: Arc::new(transports), pairer: Arc::new(paired), spawn }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn pairing(self, pairer: impl Fn(MachineCode) -> Pairing + Send + Sync + 'static) -> Self {
        Self { pairer: Arc::new(pairer), ..self }
    }

    pub(crate) fn transport(&self, route: &Route) -> Result<Arc<dyn Transport>, Error> {
        (self.transports)(route)
    }

    pub(crate) async fn pair(&self, code: &MachineCode) -> Result<Token, PairError> {
        (self.pairer)(code.clone()).await
    }
}

impl Default for Launcher {
    fn default() -> Self {
        Self::new(websocket_transports(SocketConfig::default()), async_runtime::spawn)
    }
}

fn paired(code: MachineCode) -> Pairing {
    Box::pin(async move { pair(&code, &user_agent(), PairConfig::default()).await })
}

pub(crate) fn user_agent() -> String {
    let platform = match env::consts::OS {
        "ios" => "iOS",
        "android" => "Android",
        other => other,
    };
    format!("cornercase-mobile/{} ({platform})", env!("CARGO_PKG_VERSION"))
}

#[tauri::command(async)]
#[specta::specta]
#[expect(clippy::needless_pass_by_value, reason = "tauri hands a command its arguments by value")]
pub(crate) fn connect<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>, id: u64) -> Result<(), CommandError> {
    switch_to(&state, app, &Launcher::default(), MachineId(id))
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
pub(crate) fn agents(state: State<'_, AppState>) -> Option<Vec<AgentRow>> {
    latest_agents(&state)
}

pub(crate) fn switch_to(
    state: &AppState,
    emit: impl Emit,
    launcher: &Launcher,
    id: MachineId,
) -> Result<(), CommandError> {
    if id == DEMO && state.in_demo() {
        return Ok(());
    }
    state.machines(|machines| machines.activate(id).map_err(CommandError::from))?;
    state.left_demo();
    start(state, emit, launcher, id)
}

pub(crate) fn start(state: &AppState, emit: impl Emit, launcher: &Launcher, id: MachineId) -> Result<(), CommandError> {
    let mut active = state.active();
    let machine = state.machines(|machines| machines.machine(id).cloned().ok_or(Error::UnknownMachine.into()))?;
    *active = Some(launched(state, emit, launcher, machine));
    Ok(())
}

pub(crate) fn launched(state: &AppState, emit: impl Emit, launcher: &Launcher, machine: Machine) -> Active {
    let id = machine.id;
    let transports = Arc::clone(&launcher.transports);
    let (store, driver) = Store::new(machine, move |route: &Route| transports(route), PickConfig::default());
    if state.network_is_down() && id != DEMO {
        store.offline();
    }
    let changes = store.changes();
    let forwarder = forward(id, store.clone(), changes, state.shared_machines(), emit);
    let driver = (launcher.spawn)(Box::pin(driver));
    let forwarder = (launcher.spawn)(Box::pin(forwarder));
    Active::new(id, store, driver, forwarder)
}

pub(crate) fn current(state: &AppState) -> ConnectionState {
    match state.active().as_ref() {
        Some(active) => ConnectionState::of(active.machine(), &active.store().connection()),
        None => ConnectionState::none(),
    }
}

pub(crate) fn latest_agents(state: &AppState) -> Option<Vec<AgentRow>> {
    state.active().as_ref().and_then(|active| active.store().agents())
}

#[cfg(test)]
mod tests;
