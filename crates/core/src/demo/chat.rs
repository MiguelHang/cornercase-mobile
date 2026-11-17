use std::time::Duration;

use serde_json::json;
use tokio::time::Instant;

use super::clock;
use super::world::World;
use crate::board::places;
use crate::cli::status::Tab;

pub(super) const WORKING: &str = "working";
pub(super) const DONE: &str = "done";
const IDLE: &str = "idle";
const SHORTEST: u64 = 2;
const SPREAD: u64 = 3;

impl World {
    pub(super) fn sent(&mut self, pane: u64) {
        self.worked(pane, IDLE);
    }

    pub(super) fn worked(&mut self, pane: u64, then: &'static str) {
        let thinking = Duration::from_secs(SHORTEST + (pane + self.turn(pane)) % SPREAD);
        if self.moved(pane, WORKING) {
            self.due.insert(pane, (Instant::now() + thinking, then));
        }
    }

    pub(super) fn reviewed(&mut self, pane: u64) {
        let done =
            places(&self.session).any(|place| place.pane.id == pane && place.pane.status.as_deref() == Some(DONE));
        if done {
            self.moved(pane, IDLE);
        }
    }

    pub(super) fn settle(&mut self) {
        let now = Instant::now();
        let over: Vec<(u64, &str)> =
            self.due.iter().filter(|&(_, (at, _))| *at <= now).map(|(&pane, &(_, then))| (pane, then)).collect();
        for (pane, then) in over {
            self.due.remove(&pane);
            *self.turns.entry(pane).or_default() += 1;
            self.written.insert(pane, self.now());
            self.moved(pane, then);
        }
    }

    pub(super) fn next_due(&self) -> Option<Instant> {
        self.due.values().map(|&(at, _)| at).min()
    }

    pub(super) fn last_message(&self, pane: u64) -> Option<Result<String, String>> {
        let place = places(&self.session).find(|place| place.pane.id == pane)?;
        let agent = place.pane.agent.as_deref()?;
        let Some(written) = self.written.get(&pane) else {
            return Some(Err(format!("Error: the {agent} agent in pane {pane} has not written a message yet")));
        };
        let at = usize::try_from(pane + self.turn(pane)).unwrap_or_default();
        let message = json!({
            "pane": pane,
            "agent": agent,
            "text": self.replies[at % self.replies.len()],
            "written": clock::iso(*written),
            "turn_over": !self.due.contains_key(&pane),
        });
        Some(Ok(message.to_string()))
    }

    fn turn(&self, pane: u64) -> u64 {
        self.turns.get(&pane).copied().unwrap_or_default()
    }

    fn moved(&mut self, pane: u64, to: &str) -> bool {
        let now = self.now();
        let Some(tab) = self.tabs().find(|tab| tab.panes.iter().any(|seen| seen.id == pane && seen.agent.is_some()))
        else {
            return false;
        };
        let status = Some(to.to_owned());
        tab.panes.iter_mut().filter(|seen| seen.id == pane).for_each(|seen| seen.status.clone_from(&status));
        tab.status = status;
        self.changed.insert(pane, now);
        self.revised();
        true
    }

    fn tabs(&mut self) -> impl Iterator<Item = &mut Tab> {
        let workspaces = self.session.projects.iter_mut().flat_map(|project| project.workspaces.iter_mut());
        workspaces.flat_map(|workspace| workspace.tabs.iter_mut())
    }
}

#[cfg(test)]
mod tests;
