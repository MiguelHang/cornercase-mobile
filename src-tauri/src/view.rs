use std::time::Duration;

use cornercase_core::cli::contract::Missing;
use cornercase_core::machines::route::Route;
use cornercase_core::machines::{Machine, MachineId};
use cornercase_core::store::Connection;
use serde::Serialize;

use crate::commands::error::CommandError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct MachineView {
    #[specta(type = u32)]
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) routes: Vec<RouteView>,
    pub(crate) last_route: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum RouteView {
    Bridge { url: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum ConnectionView {
    None,
    Connecting,
    Connected {
        route: String,
    },
    Retrying {
        #[specta(type = u32)]
        in_ms: u64,
        error: CommandError,
    },
    Missing {
        missing: Vec<Missing>,
    },
    Unauthorized,
    Incompatible {
        error: CommandError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct ConnectionState {
    #[specta(type = Option<u32>)]
    pub(crate) machine: Option<u64>,
    pub(crate) connection: ConnectionView,
}

impl From<&Machine> for MachineView {
    fn from(machine: &Machine) -> Self {
        Self {
            id: machine.id.0,
            name: machine.name.clone(),
            routes: machine.routes.iter().map(RouteView::from).collect(),
            last_route: machine.last_route.as_ref().map(ToString::to_string),
        }
    }
}

impl From<&Route> for RouteView {
    fn from(route: &Route) -> Self {
        match route {
            Route::Bridge { .. } => Self::Bridge { url: route.address().to_string() },
        }
    }
}

impl From<&Connection> for ConnectionView {
    fn from(connection: &Connection) -> Self {
        match connection {
            Connection::Connecting => Self::Connecting,
            Connection::Connected { route } => Self::Connected { route: route.to_string() },
            Connection::Retrying { r#in, error } => Self::Retrying { in_ms: millis(*r#in), error: error.into() },
            Connection::Missing(missing) => Self::Missing { missing: missing.clone() },
            Connection::Unauthorized => Self::Unauthorized,
            Connection::Incompatible(error) => Self::Incompatible { error: error.into() },
        }
    }
}

impl ConnectionState {
    pub(crate) fn none() -> Self {
        Self { machine: None, connection: ConnectionView::None }
    }

    pub(crate) fn of(machine: MachineId, connection: &Connection) -> Self {
        Self { machine: Some(machine.0), connection: connection.into() }
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
