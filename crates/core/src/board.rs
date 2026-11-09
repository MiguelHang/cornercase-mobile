use std::collections::{HashMap, HashSet};

use crate::cli::status::{Pane, Snapshot};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Since {
    panes: HashMap<u64, Heard>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Heard {
    status: Option<String>,
    time: Option<String>,
}

impl Since {
    pub fn heard(&mut self, pane: u64, to: Option<&str>, time: &str) {
        let heard = Heard { status: to.map(str::to_owned), time: Some(time.to_owned()) };
        self.panes.insert(pane, heard);
    }

    pub fn reconcile(&mut self, snapshot: &Snapshot) {
        let present: HashSet<u64> = panes(snapshot).map(|pane| pane.id).collect();
        self.panes.retain(|pane, _| present.contains(pane));
        for pane in panes(snapshot).filter(|pane| pane.agent.is_some()) {
            self.see(pane);
        }
    }

    pub fn clear(&mut self) {
        self.panes.clear();
    }

    pub fn of(&self, pane: u64, status: Option<&str>) -> Option<&str> {
        let heard = self.panes.get(&pane)?;
        if heard.status.as_deref() == status { heard.time.as_deref() } else { None }
    }

    fn see(&mut self, pane: &Pane) {
        let heard = self.panes.entry(pane.id).or_default();
        if heard.status != pane.status {
            *heard = Heard { status: pane.status.clone(), time: None };
        }
    }
}

fn panes(snapshot: &Snapshot) -> impl Iterator<Item = &Pane> {
    snapshot
        .projects
        .iter()
        .flat_map(|project| &project.workspaces)
        .flat_map(|workspace| &workspace.tabs)
        .flat_map(|tab| &tab.panes)
}

#[cfg(test)]
mod tests;
