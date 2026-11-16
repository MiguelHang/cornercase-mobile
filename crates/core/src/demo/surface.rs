use serde::Deserialize;
use serde_json::{Value, json};

use super::world::World;
use crate::board::places;
use crate::cli::surface::Size;

pub(super) type Span = (Tone, String);
pub(super) type Row = Vec<Span>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Tone {
    Plain,
    Dim,
    Chip,
    Asked,
    Picked,
    Bold,
    Accent,
    Green,
    Red,
    Yellow,
    Blue,
    Magenta,
    Cyan,
}

const TONES: [Tone; 13] = [
    Tone::Plain,
    Tone::Dim,
    Tone::Chip,
    Tone::Asked,
    Tone::Picked,
    Tone::Bold,
    Tone::Accent,
    Tone::Green,
    Tone::Red,
    Tone::Yellow,
    Tone::Blue,
    Tone::Magenta,
    Tone::Cyan,
];

impl Tone {
    fn index(self) -> usize {
        TONES.iter().position(|tone| *tone == self).unwrap_or_default()
    }

    fn style(self) -> Value {
        match self {
            Self::Plain => json!({}),
            Self::Dim => fg(246),
            Self::Chip => json!({ "fg": { "index": 16 }, "bg": { "index": 153 } }),
            Self::Asked => json!({ "fg": { "index": 231 }, "bold": true }),
            Self::Picked => fg(153),
            Self::Bold => json!({ "bold": true }),
            Self::Accent => fg(209),
            Self::Green => fg(114),
            Self::Red => fg(174),
            Self::Yellow => fg(179),
            Self::Blue => fg(110),
            Self::Magenta => fg(176),
            Self::Cyan => fg(116),
        }
    }
}

pub(super) fn row(tone: Tone, text: impl Into<String>) -> Row {
    vec![(tone, text.into())]
}

const SESSION: u64 = 1;

pub(super) struct Surface {
    tab: u64,
    view: Option<u64>,
    size: Size,
    revision: u64,
    shown: Option<Vec<Value>>,
}

impl Surface {
    pub(super) fn new(tab: u64, view: Option<u64>, size: Size) -> Self {
        Self { tab, view, size, revision: 0, shown: None }
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
            "view": self.view,
            "cols": cols,
            "rows": rows,
            "layout": { "leaf": first },
            "styles": TONES.map(Tone::style),
            "hyperlinks": [],
            "panes": drawn,
        });
        self.shown = Some(drawn);
        Ok(vec![frame.to_string()])
    }

    fn pane(&self, pane: u64, active: bool, screen: &[Row]) -> Value {
        let Size { cols, rows } = self.size;
        let hidden = screen.len().saturating_sub(usize::from(rows));
        let lines: Vec<Value> = screen.iter().skip(hidden).map(|spans| line(spans, usize::from(cols))).collect();
        let area = json!({ "x": 0, "y": 0, "cols": cols, "rows": rows });
        json!({ "pane": pane, "active": active, "rect": area, "size": { "cols": cols, "rows": rows }, "lines": lines })
    }
}

fn line(spans: &[Span], cols: usize) -> Value {
    let mut left = cols;
    let mut runs = Vec::new();
    for (tone, text) in spans {
        let cells: Vec<String> = text.chars().take(left).map(String::from).collect();
        left -= cells.len();
        if !cells.is_empty() {
            runs.push(json!({ "style": tone.index(), "cells": cells }));
        }
    }
    Value::Array(runs)
}

fn fg(index: u8) -> Value {
    json!({ "fg": { "index": index } })
}
