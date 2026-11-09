use crate::cli::status::{Pane, Project, Snapshot, Tab, Workspace};

pub(crate) fn pane(id: u64, agent: Option<&str>, status: Option<&str>) -> Pane {
    Pane { id, agent: agent.map(str::to_owned), status: status.map(str::to_owned), ..Pane::default() }
}

pub(crate) fn agent(id: u64, status: &str) -> Pane {
    pane(id, Some("claude"), Some(status))
}

pub(crate) fn child(id: u64, parent: u64, status: &str) -> Pane {
    Pane { parent: Some(parent), ..agent(id, status) }
}

pub(crate) fn member(id: u64, parent: u64, role: &str) -> Pane {
    Pane { role: Some(role.to_owned()), ..child(id, parent, "idle") }
}

pub(crate) fn tab(id: u64, panes: Vec<Pane>) -> Tab {
    Tab { id, name: format!("tab {id}"), panes, ..Tab::default() }
}

pub(crate) fn workspace(id: u64, tabs: Vec<Tab>) -> Workspace {
    Workspace { id, name: format!("workspace {id}"), tabs, ..Workspace::default() }
}

pub(crate) fn project(id: u64, workspaces: Vec<Workspace>) -> Project {
    Project { id, name: format!("project {id}"), workspaces, ..Project::default() }
}

pub(crate) fn session(projects: Vec<Project>) -> Snapshot {
    Snapshot { boot: "1-2".to_owned(), projects, ..Snapshot::default() }
}

pub(crate) fn snapshot(panes: Vec<Pane>) -> Snapshot {
    session(vec![project(1, vec![workspace(2, vec![tab(3, panes)])])])
}
