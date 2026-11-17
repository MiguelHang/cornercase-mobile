use std::collections::HashMap;
use std::iter;

use super::dispatch::{Answer, flagged, json, valued};
use super::world::World;
use crate::board::{Place, places};
use crate::cli::close::{AgentClosing, PaneRef};

pub(super) fn answer(world: &mut World, pane: u64, argv: &[String]) -> Answer {
    let asked = flagged(argv, "--agent") && flagged(argv, "--json");
    let same_boot = valued(argv, "--boot") == Some(world.session.boot.as_str());
    let Some(closing) = world.closing(pane).filter(|_| asked && same_boot) else { return Answer::unknown(argv) };
    let closed = !flagged(argv, "--check");
    if closed {
        world.closed(&closing.tabs);
    }
    Answer::printed(json(&AgentClosing { closed, ..closing }))
}

fn reference(place: &Place) -> PaneRef {
    PaneRef {
        agent: place.pane.agent.clone(),
        pane: place.pane.id,
        project: place.project.id,
        tab: place.tab.id,
        workspace: place.workspace.id,
    }
}

impl World {
    fn closing(&self, pane: u64) -> Option<AgentClosing> {
        let root = places(&self.session).find(|place| place.pane.id == pane)?;
        let agent = root.pane.agent.clone()?;
        let parents: HashMap<u64, u64> =
            places(&self.session).filter_map(|place| Some((place.pane.id, place.pane.parent?))).collect();
        let under = |place: &Place| {
            iter::successors(Some(place.pane.id), |seen| parents.get(seen).copied()).any(|seen| seen == pane)
        };
        let stops: Vec<PaneRef> = places(&self.session).filter(under).map(|place| reference(&place)).collect();
        let mut tabs: Vec<u64> = stops.iter().map(|stop| stop.tab).collect();
        tabs.dedup();
        Some(AgentClosing {
            pane,
            agent,
            project: root.project.id,
            workspace: root.workspace.id,
            tabs,
            stops,
            ..AgentClosing::default()
        })
    }

    fn closed(&mut self, tabs: &[u64]) {
        let gone: Vec<u64> =
            places(&self.session).filter(|place| tabs.contains(&place.tab.id)).map(|place| place.pane.id).collect();
        let workspaces = self.session.projects.iter_mut().flat_map(|project| project.workspaces.iter_mut());
        workspaces.for_each(|workspace| workspace.tabs.retain(|tab| !tabs.contains(&tab.id)));
        self.due.retain(|pane, _| !gone.contains(pane));
        self.printed.retain(|pane, _| !gone.contains(pane));
        self.revised();
    }
}

#[cfg(test)]
mod tests;
