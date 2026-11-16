use serde_json::{Value, json};

use super::surface::Row;
use super::world::World;
use crate::board::places;
use crate::cli::surface::Size;

const PROJECTION: u64 = 1;

impl World {
    pub(super) fn screen(&self, pane: u64, cols: u16) -> Vec<Row> {
        self.asked(pane, cols).or_else(|| self.printed.get(&pane).cloned()).unwrap_or_default()
    }

    pub(super) fn view(&self, tab: u64, size: Size) -> String {
        let mut line = serde_json::to_value(&self.session).expect("a snapshot always serializes");
        line["view"] = json!({
            "id": tab,
            "projection": PROJECTION,
            "active": true,
            "cols": size.cols,
            "rows": size.rows,
            "shown": self.shown(tab),
            "controls": true,
            "visible": true,
        });
        line.to_string()
    }

    fn shown(&self, tab: u64) -> Value {
        places(&self.session).find(|place| place.tab.id == tab).map_or_else(
            || json!({}),
            |place| json!({ "project": place.project.id, "workspace": place.workspace.id, "tab": tab, "pane": place.pane.id }),
        )
    }
}

#[cfg(test)]
mod tests;
