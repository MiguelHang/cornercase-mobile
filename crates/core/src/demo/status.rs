use std::collections::HashMap;

use serde_json::json;

use super::clock;
use super::world::World;
use crate::board::{Place, places};

impl World {
    pub(super) fn status(&self) -> String {
        serde_json::to_string(&self.session).expect("a snapshot always serializes")
    }

    pub(super) fn app(&self) -> String {
        let session = &self.session;
        json!({ "revision": session.revision, "version": session.version, "boot": session.boot, "todos": [], "queue": [] })
            .to_string()
    }

    pub(super) fn events(&self, told: &mut HashMap<u64, String>) -> Vec<String> {
        let changes = places(&self.session).filter_map(|place| Some((place.pane.id, self.changed(&place)?)));
        changes
            .filter(|(pane, event)| told.insert(*pane, event.clone()).as_ref() != Some(event))
            .map(|(_, event)| event)
            .collect()
    }

    fn changed(&self, place: &Place) -> Option<String> {
        let (agent, changed) = (place.pane.agent.as_deref()?, self.changed.get(&place.pane.id)?);
        let event = json!({
            "time": clock::iso(*changed),
            "event": "status",
            "agent": agent,
            "from": null,
            "to": place.pane.status,
            "project": place.project.id,
            "workspace": place.workspace.id,
            "tab": place.tab.id,
            "pane": place.pane.id,
        });
        Some(event.to_string())
    }
}
