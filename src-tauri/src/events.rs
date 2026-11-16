use std::collections::BTreeMap;
use std::future::{self, Future};
use std::pin::Pin;
use std::sync::Arc;

use cornercase_core::board::AgentRow;
use cornercase_core::cli::changes::{Mode, Summary};
use cornercase_core::cli::events::{Event as CliEvent, What};
use cornercase_core::cli::surface::{Colours, Cursor, Modes, Size};
use cornercase_core::error::Error;
use cornercase_core::machines::MachineId;
use cornercase_core::machines::learned::ask_routes;
use cornercase_core::machines::registry::Machines;
use cornercase_core::machines::route::{Address, Route};
use cornercase_core::store::{Change, Changes, Connection, Store};
use cornercase_core::terminal::grid::Line;
use cornercase_core::terminal::question::{Card, Choice};
use cornercase_core::transport::Transport;
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

use crate::commands::error::CommandError;
use crate::state::{DEMO, SharedMachines};
use crate::view::ConnectionView;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type, Event)]
pub(crate) struct ConnectionChanged {
    #[specta(type = u32)]
    pub(crate) machine: u64,
    pub(crate) connection: ConnectionView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type, Event)]
pub(crate) struct AgentsChanged {
    #[specta(type = u32)]
    pub(crate) machine: u64,
    pub(crate) agents: Option<Vec<AgentRow>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type, Event)]
pub(crate) struct PaneStatusChanged {
    #[specta(type = u32)]
    pub(crate) machine: u64,
    #[specta(type = Option<u32>)]
    pub(crate) pane: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type, Event)]
pub(crate) struct TerminalChanged {
    #[specta(type = u32)]
    pub(crate) machine: u64,
    #[specta(type = u32)]
    pub(crate) tab: u64,
    #[specta(type = Option<u32>)]
    pub(crate) pane: Option<u64>,
    pub(crate) size: Size,
    pub(crate) cursor: Option<Cursor>,
    pub(crate) colours: Colours,
    pub(crate) modes: Modes,
    pub(crate) replaced: bool,
    pub(crate) rows: BTreeMap<u16, Line>,
    pub(crate) closed: bool,
    pub(crate) unfitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type, Event)]
pub(crate) struct QuestionChanged {
    #[specta(type = u32)]
    pub(crate) machine: u64,
    #[specta(type = u32)]
    pub(crate) tab: u64,
    #[specta(type = u32)]
    pub(crate) pane: u64,
    pub(crate) colours: Colours,
    pub(crate) rows: Vec<Line>,
    pub(crate) options: Vec<Choice>,
    pub(crate) card: Option<Card>,
    pub(crate) closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type, Event)]
pub(crate) struct ChangesChanged {
    #[specta(type = u32)]
    pub(crate) machine: u64,
    #[specta(type = u32)]
    pub(crate) workspace: u64,
    pub(crate) mode: Mode,
    pub(crate) summary: Option<Summary>,
    pub(crate) closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type, Event)]
pub(crate) struct Toast {
    pub(crate) level: ToastLevel,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ToastLevel {
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Emitted {
    Connection(ConnectionChanged),
    Agents(AgentsChanged),
    PaneStatus(PaneStatusChanged),
    Terminal(TerminalChanged),
    Question(QuestionChanged),
    Changes(ChangesChanged),
    Toast(Toast),
}

pub(crate) trait Emit: Send + Sync + 'static {
    fn emit(&self, event: Emitted);
}

impl<R: Runtime> Emit for AppHandle<R> {
    fn emit(&self, event: Emitted) {
        let sent = match event {
            Emitted::Connection(changed) => changed.emit(self),
            Emitted::Agents(changed) => changed.emit(self),
            Emitted::PaneStatus(changed) => changed.emit(self),
            Emitted::Terminal(changed) => changed.emit(self),
            Emitted::Question(changed) => changed.emit(self),
            Emitted::Changes(changed) => changed.emit(self),
            Emitted::Toast(toast) => toast.emit(self),
        };
        sent.ok();
    }
}

impl Toast {
    pub(crate) fn info(message: impl Into<String>) -> Self {
        Self { level: ToastLevel::Info, message: message.into() }
    }

    pub(crate) fn error(message: impl Into<String>) -> Self {
        Self { level: ToastLevel::Error, message: message.into() }
    }
}

pub(crate) async fn forward(
    machine: MachineId,
    store: Store,
    mut changes: Changes,
    machines: SharedMachines,
    emit: impl Emit,
) {
    let mut sent = None;
    let mut learning = None;
    loop {
        let change = tokio::select! {
            change = changes.next() => change,
            () = learned(&mut learning), if learning.is_some() => {
                learning = None;
                continue;
            }
        };
        let Some(change) = change else {
            return;
        };
        learning = asking_routes(machine, &store, &machines, &change).or(learning);
        let agents = if touches_agents(&change) { agents_changed(machine, &store, &mut sent) } else { None };
        for event in translated(machine, &machines, change).into_iter().chain(agents) {
            emit.emit(event);
        }
    }
}

type Learning = Pin<Box<dyn Future<Output = ()> + Send>>;

async fn learned(learning: &mut Option<Learning>) {
    match learning {
        Some(learning) => learning.await,
        None => future::pending().await,
    }
}

fn asking_routes(machine: MachineId, store: &Store, machines: &SharedMachines, change: &Change) -> Option<Learning> {
    let Change::Connection(Connection::Connected { route }) = change else {
        return None;
    };
    let winner = machines.with(|all| Ok(saved_route(all, machine, route))).ok().flatten()?;
    let transport = store.transport()?;
    Some(Box::pin(learn(machine, transport, winner, store.clone(), machines.clone())))
}

fn saved_route(machines: &Machines, id: MachineId, address: &Address) -> Option<Route> {
    machines.machine(id)?.routes.iter().find(|route| route.address() == address).cloned()
}

async fn learn(
    machine: MachineId,
    transport: Arc<dyn Transport>,
    winner: Route,
    store: Store,
    machines: SharedMachines,
) {
    let routes = match ask_routes(transport.as_ref(), &winner).await {
        Ok(routes) => routes,
        Err(error) => {
            eprintln!("cornercase-mobile: the machine did not tell its routes: {error}");
            return;
        }
    };
    match machines.with(|all| all.learn(machine, &routes).map_err(CommandError::from)) {
        Ok(new) => store.learn(&new),
        Err(error) => eprintln!("cornercase-mobile: the routes the machine told were not saved: {}", error.message),
    }
}

fn touches_agents(change: &Change) -> bool {
    match change {
        Change::Session => true,
        Change::Event(event) => matches!(event.what, What::Status { .. } | What::Dropped { .. }),
        Change::Connection(_) | Change::App | Change::Reopened | Change::Warning(_) => false,
    }
}

fn agents_changed(machine: MachineId, store: &Store, sent: &mut Option<Vec<AgentRow>>) -> Option<Emitted> {
    let agents = store.agents();
    if agents == *sent {
        return None;
    }
    sent.clone_from(&agents);
    Some(Emitted::Agents(AgentsChanged { machine: machine.0, agents }))
}

fn translated(machine: MachineId, machines: &SharedMachines, change: Change) -> Vec<Emitted> {
    match change {
        Change::Connection(connection) => connection_changed(machine, machines, &connection),
        Change::Warning(error) => vec![warned(&error)],
        Change::Event(event) => pane_status_changed(machine, &event).into_iter().collect(),
        Change::Reopened => vec![Emitted::PaneStatus(PaneStatusChanged { machine: machine.0, pane: None })],
        Change::Session | Change::App => Vec::new(),
    }
}

fn pane_status_changed(machine: MachineId, event: &CliEvent) -> Option<Emitted> {
    let pane = match event.what {
        What::Status { .. } => Some(event.pane?),
        What::Dropped { .. } => None,
        What::Opened { .. } | What::Closed { .. } | What::Exited { .. } | What::Notice { .. } | What::Other(_) => {
            return None;
        }
    };
    Some(Emitted::PaneStatus(PaneStatusChanged { machine: machine.0, pane }))
}

fn connection_changed(machine: MachineId, machines: &SharedMachines, connection: &Connection) -> Vec<Emitted> {
    let changed = Emitted::Connection(ConnectionChanged { machine: machine.0, connection: connection.into() });
    let (Connection::Connected { route }, false) = (connection, machine == DEMO) else {
        return vec![changed];
    };
    match machines.with(|all| all.remember(machine, route).map_err(CommandError::from)) {
        Ok(()) => vec![changed],
        Err(error) => vec![changed, Emitted::Toast(Toast::error(error.message))],
    }
}

pub(crate) fn warned(error: &Error) -> Emitted {
    Emitted::Toast(Toast::error(error.to_string()))
}

#[cfg(test)]
mod tests;
