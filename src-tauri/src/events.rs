use cornercase_core::cli::status::Snapshot;
use cornercase_core::error::Error;
use cornercase_core::machines::MachineId;
use cornercase_core::store::{Change, Changes, Connection, Store};
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
pub(crate) struct SessionChanged {
    #[specta(type = u32)]
    pub(crate) machine: u64,
    pub(crate) session: Option<Snapshot>,
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
    Session(SessionChanged),
    Toast(Toast),
}

pub(crate) trait Emit: Send + Sync + 'static {
    fn emit(&self, event: Emitted);
}

impl<R: Runtime> Emit for AppHandle<R> {
    fn emit(&self, event: Emitted) {
        let sent = match event {
            Emitted::Connection(changed) => changed.emit(self),
            Emitted::Session(changed) => changed.emit(self),
            Emitted::Toast(toast) => toast.emit(self),
        };
        sent.ok();
    }
}

impl Toast {
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
    while let Some(change) = changes.next().await {
        for event in translated(machine, &store, &machines, change) {
            emit.emit(event);
        }
    }
}

fn translated(machine: MachineId, store: &Store, machines: &SharedMachines, change: Change) -> Vec<Emitted> {
    match change {
        Change::Connection(connection) => connection_changed(machine, machines, &connection),
        Change::Session => vec![Emitted::Session(SessionChanged { machine: machine.0, session: store.session() })],
        Change::Warning(error) => vec![warned(&error)],
        Change::Event(_) | Change::App => Vec::new(),
    }
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

fn warned(error: &Error) -> Emitted {
    Emitted::Toast(Toast::error(error.to_string()))
}

#[cfg(test)]
mod tests;
