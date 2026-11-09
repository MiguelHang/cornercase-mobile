use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::cli::status::{Pane, Project, Snapshot, Tab, Workspace};

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AgentRow {
    pub boot: String,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub workspace: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub tab: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub pane: u64,
    pub project_name: String,
    pub workspace_name: String,
    pub tab_name: String,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub workspace_count: usize,
    pub agent: String,
    pub title: Option<String>,
    pub status: Option<String>,
    pub since: Option<String>,
}

pub(crate) fn agents(snapshot: &Snapshot, since: &Since) -> Vec<AgentRow> {
    let mut rows: Vec<AgentRow> = places(snapshot).filter_map(|place| place.row(since)).collect();
    rows.sort_by(|a, b| {
        let ranks = rank(b.status.as_deref()).cmp(&rank(a.status.as_deref()));
        ranks.then_with(|| a.since.cmp(&b.since))
    });
    rows
}

fn agent_title(title: Option<&str>) -> Option<String> {
    let title = title?.trim_start_matches(|c: char| !c.is_alphanumeric()).trim();
    (!title.is_empty()).then(|| title.to_owned())
}

fn rank(status: Option<&str>) -> u8 {
    match status {
        Some("waiting") => 5,
        Some("failed") => 4,
        Some("done") => 3,
        Some("pending") => 2,
        Some("working" | "shell") => 1,
        _ => 0,
    }
}

struct Place<'a> {
    boot: &'a str,
    project: &'a Project,
    workspace: &'a Workspace,
    tab: &'a Tab,
    pane: &'a Pane,
}

impl Place<'_> {
    fn row(&self, since: &Since) -> Option<AgentRow> {
        let agent = self.pane.agent.clone()?;
        let status = self.pane.status.clone();
        Some(AgentRow {
            boot: self.boot.to_owned(),
            project: self.project.id,
            workspace: self.workspace.id,
            tab: self.tab.id,
            pane: self.pane.id,
            project_name: shown(&self.project.name, self.project.custom_name.as_deref()),
            workspace_name: shown(&self.workspace.name, self.workspace.custom_name.as_deref()),
            tab_name: shown(&self.tab.name, self.tab.custom_name.as_deref()),
            workspace_count: self.project.workspaces.len(),
            agent,
            title: agent_title(self.pane.title.as_deref()),
            since: since.of(self.pane.id, status.as_deref()).map(str::to_owned),
            status,
        })
    }
}

fn shown(name: &str, custom_name: Option<&str>) -> String {
    custom_name.unwrap_or(name).to_owned()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Since {
    panes: HashMap<u64, Heard>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Heard {
    status: Option<String>,
    time: Option<String>,
    ahead: Option<Ahead>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Ahead {
    status: Option<String>,
    time: String,
}

impl Heard {
    fn unseen(&self) -> bool {
        self.status.is_none() && self.time.is_none() && self.ahead.is_some()
    }
}

impl Since {
    pub(crate) fn heard(&mut self, pane: u64, to: Option<&str>, time: &str) {
        match self.panes.get_mut(&pane) {
            Some(heard) if heard.status.as_deref() == to => {
                heard.time = Some(time.to_owned());
                heard.ahead = None;
            }
            _ => {
                let ahead = Ahead { status: to.map(str::to_owned), time: time.to_owned() };
                self.panes.entry(pane).or_default().ahead = Some(ahead);
            }
        }
    }

    pub(crate) fn reconcile(&mut self, snapshot: &Snapshot) {
        let present: HashSet<u64> = places(snapshot).map(|place| place.pane.id).collect();
        self.panes.retain(|pane, heard| present.contains(pane) || heard.unseen());
        for place in places(snapshot).filter(|place| place.pane.agent.is_some()) {
            self.see(place.pane);
        }
    }

    pub(crate) fn doubt(&mut self) -> bool {
        let mut doubted = false;
        for heard in self.panes.values_mut() {
            doubted |= heard.time.take().is_some() | heard.ahead.take().is_some();
        }
        doubted
    }

    pub(crate) fn clear(&mut self) {
        self.panes.clear();
    }

    pub(crate) fn of(&self, pane: u64, status: Option<&str>) -> Option<&str> {
        let heard = self.panes.get(&pane)?;
        if heard.status.as_deref() == status { heard.time.as_deref() } else { None }
    }

    fn see(&mut self, pane: &Pane) {
        let heard = self.panes.entry(pane.id).or_default();
        if let Some(ahead) = heard.ahead.take_if(|ahead| ahead.status == pane.status) {
            *heard = Heard { status: ahead.status, time: Some(ahead.time), ahead: None };
        } else if heard.status != pane.status {
            heard.status.clone_from(&pane.status);
            heard.time = None;
        }
    }
}

fn places(snapshot: &Snapshot) -> impl Iterator<Item = Place<'_>> {
    let boot = snapshot.boot.as_str();
    snapshot
        .projects
        .iter()
        .flat_map(|project| project.workspaces.iter().map(move |workspace| (project, workspace)))
        .flat_map(|(project, workspace)| workspace.tabs.iter().map(move |tab| (project, workspace, tab)))
        .flat_map(move |(project, workspace, tab)| {
            tab.panes.iter().map(move |pane| Place { boot, project, workspace, tab, pane })
        })
}

#[cfg(test)]
mod tests;
