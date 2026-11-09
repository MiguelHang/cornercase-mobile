use std::collections::BTreeMap;

use cornercase_core::board::AgentRow;
use cornercase_core::cli::changes::{Mode, Summary};
use cornercase_core::cli::events::{Event as CliEvent, What};
use cornercase_core::cli::surface::{Colours, Cursor, Modes, Size};
use cornercase_core::error::Error;
use cornercase_core::machines::MachineId;
use cornercase_core::store::{Change, Changes, Connection, Store};
use cornercase_core::terminal::grid::Line;
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

use crate::commands::error::CommandError;
use crate::state::SharedMachines;
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
    while let Some(change) = changes.next().await {
        let agents = if touches_agents(&change) { agents_changed(machine, &store, &mut sent) } else { None };
        for event in translated(machine, &machines, change).into_iter().chain(agents) {
            emit.emit(event);
        }
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
    let Connection::Connected { route } = connection else {
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
