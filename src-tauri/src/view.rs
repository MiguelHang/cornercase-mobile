use std::time::Duration;

use cornercase_core::cli::contract::{Kind, Missing};
use cornercase_core::cli::status::{Project, Workspace};
use cornercase_core::demo;
use cornercase_core::machines::registry::{Machines, Merged};
use cornercase_core::machines::route::Route;
use cornercase_core::machines::{Machine, MachineId};
use cornercase_core::store::Connection;
use serde::Serialize;

use crate::commands::error::CommandError;
use crate::state::DEMO;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct MachineView {
    #[specta(type = u32)]
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) routes: Vec<RouteView>,
    pub(crate) last_route: Option<String>,
    pub(crate) demo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct MachineList {
    pub(crate) machines: Vec<MachineView>,
    #[specta(type = Option<u32>)]
    pub(crate) active: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum RouteView {
    Cli { url: String, has_token: bool },
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
        missing: Vec<MissingView>,
    },
    NotPaired,
    Incompatible {
        error: CommandError,
    },
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct MissingView {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) reason: Option<String>,
    pub(crate) since: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum ProbeView {
    Answers { route: String },
    NotPaired,
    NoToken,
    NoAnswer { failures: Vec<RouteFailure> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct RouteFailure {
    pub(crate) route: String,
    pub(crate) error: CommandError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct MatchView {
    pub(crate) found: MatchFound,
    pub(crate) checked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum MatchFound {
    None,
    Same {
        #[specta(type = u32)]
        machine: u64,
        name: String,
        why: SameWhy,
    },
    Merge {
        #[specta(type = u32)]
        machine: u64,
        name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SameWhy {
    Address,
    Boot,
    Name,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct MergeView {
    pub(crate) added: Vec<String>,
    pub(crate) token_updated: bool,
    pub(crate) skipped: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct ConnectionState {
    #[specta(type = Option<u32>)]
    pub(crate) machine: Option<u64>,
    pub(crate) connection: ConnectionView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct ProjectChoice {
    #[specta(type = u32)]
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) workspaces: Vec<WorkspaceChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct WorkspaceChoice {
    #[specta(type = u32)]
    pub(crate) id: u64,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct StartChoices {
    pub(crate) boot: Option<String>,
    pub(crate) projects: Vec<ProjectChoice>,
}

impl From<&Machine> for MachineView {
    fn from(machine: &Machine) -> Self {
        Self {
            id: machine.id.0,
            name: machine.name.clone(),
            routes: machine.routes.iter().map(RouteView::from).collect(),
            last_route: machine.last_route.as_ref().map(ToString::to_string),
            demo: false,
        }
    }
}

impl MachineView {
    fn demo() -> Self {
        Self { id: DEMO.0, name: demo::NAME.to_owned(), routes: Vec::new(), last_route: None, demo: true }
    }
}

impl MachineList {
    #[must_use]
    pub(crate) fn with_demo(mut self) -> Self {
        self.machines.push(MachineView::demo());
        self.active = Some(DEMO.0);
        self
    }
}

impl From<&Machines> for MachineList {
    fn from(machines: &Machines) -> Self {
        Self {
            machines: machines.machines().iter().map(MachineView::from).collect(),
            active: machines.active().map(|id| id.0),
        }
    }
}

impl From<&Route> for RouteView {
    fn from(route: &Route) -> Self {
        Self::Cli { url: route.address().to_string(), has_token: route.has_token() }
    }
}

impl From<&Merged> for MergeView {
    fn from(merged: &Merged) -> Self {
        Self {
            added: merged.added.iter().map(ToString::to_string).collect(),
            token_updated: merged.token_updated,
            skipped: merged.skipped.iter().map(ToString::to_string).collect(),
        }
    }
}

impl From<&Connection> for ConnectionView {
    fn from(connection: &Connection) -> Self {
        match connection {
            Connection::Connecting => Self::Connecting,
            Connection::Connected { route } => Self::Connected { route: route.to_string() },
            Connection::Retrying { r#in, error } => Self::Retrying { in_ms: millis(*r#in), error: error.into() },
            Connection::Missing(missing) => Self::Missing { missing: missing.iter().map(MissingView::from).collect() },
            Connection::NotPaired => Self::NotPaired,
            Connection::Incompatible(error) => Self::Incompatible { error: error.into() },
            Connection::Offline => Self::Offline,
        }
    }
}

impl ConnectionView {
    pub(crate) fn of(machine: MachineId, connection: &Connection) -> Self {
        match (connection, machine == DEMO) {
            (Connection::Connected { .. }, true) => Self::Connected { route: String::new() },
            _ => connection.into(),
        }
    }
}

impl From<&Missing> for MissingView {
    fn from(missing: &Missing) -> Self {
        Self {
            name: missing.name.clone(),
            kind: missing.kind,
            reason: missing.reason.clone(),
            since: missing.since().map(ToOwned::to_owned),
        }
    }
}

impl From<&Project> for ProjectChoice {
    fn from(project: &Project) -> Self {
        Self {
            id: project.id,
            name: project.custom_name.clone().unwrap_or_else(|| project.name.clone()),
            workspaces: project.workspaces.iter().map(WorkspaceChoice::from).collect(),
        }
    }
}

impl From<&Workspace> for WorkspaceChoice {
    fn from(workspace: &Workspace) -> Self {
        Self { id: workspace.id, name: workspace.custom_name.clone().unwrap_or_else(|| workspace.name.clone()) }
    }
}

impl ConnectionState {
    pub(crate) fn none() -> Self {
        Self { machine: None, connection: ConnectionView::None }
    }

    pub(crate) fn of(machine: MachineId, connection: &Connection) -> Self {
        Self { machine: Some(machine.0), connection: ConnectionView::of(machine, connection) }
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
