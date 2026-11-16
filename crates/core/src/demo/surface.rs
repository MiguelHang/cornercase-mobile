use serde_json::{Value, json};

use super::world::World;
use crate::board::places;
use crate::cli::surface::Size;

pub(super) type Row = (usize, String);

pub(super) const PLAIN: usize = 0;
pub(super) const DIM: usize = 1;
pub(super) const CHIP: usize = 2;
pub(super) const ASKED: usize = 3;
pub(super) const PICKED: usize = 4;

const SESSION: u64 = 1;

pub(super) struct Surface {
    tab: u64,
    size: Size,
    revision: u64,
    shown: Option<Vec<Value>>,
}

impl Surface {
    pub(super) fn new(tab: u64, size: Size) -> Self {
        Self { tab, size, revision: 0, shown: None }
    }

    pub(super) fn fresh(&mut self, world: &World) -> Result<Vec<String>, String> {
        let panes: Vec<u64> =
            places(&world.session).filter(|place| place.tab.id == self.tab).map(|place| place.pane.id).collect();
        let first = *panes.first().ok_or_else(|| format!("error: no tab {}", self.tab))?;
        let drawn: Vec<Value> =
            panes.iter().map(|&pane| self.pane(pane, pane == first, &world.screen(pane, self.size.cols))).collect();
        if self.shown.as_ref() == Some(&drawn) {
            return Ok(Vec::new());
        }
        self.revision += 1;
        let Size { cols, rows } = self.size;
        let frame = json!({
            "boot": world.session.boot,
            "session": SESSION,
            "revision": self.revision,
            "surface": self.tab,
            "tab": self.tab,
            "cols": cols,
            "rows": rows,
            "layout": { "leaf": first },
            "styles": styles(),
            "hyperlinks": [],
            "panes": drawn,
        });
        self.shown = Some(drawn);
        Ok(vec![frame.to_string()])
    }

    fn pane(&self, pane: u64, active: bool, screen: &[Row]) -> Value {
        let Size { cols, rows } = self.size;
        let hidden = screen.len().saturating_sub(usize::from(rows));
        let lines: Vec<Value> = screen.iter().skip(hidden).map(|(style, text)| line(*style, text, cols)).collect();
        let area = json!({ "x": 0, "y": 0, "cols": cols, "rows": rows });
        json!({ "pane": pane, "active": active, "rect": area, "size": { "cols": cols, "rows": rows }, "lines": lines })
    }
}

fn line(style: usize, text: &str, cols: u16) -> Value {
    let cells: Vec<String> = text.chars().take(usize::from(cols)).map(String::from).collect();
    if cells.is_empty() {
        return json!([]);
    }
    json!([{ "style": style, "cells": cells }])
}

fn styles() -> Value {
    json!([
        {},
        { "fg": { "index": 246 } },
        { "fg": { "index": 16 }, "bg": { "index": 153 } },
        { "fg": { "index": 231 }, "bold": true },
        { "fg": { "index": 153 } },
    ])
}
