use serde::Deserialize;

use crate::cli::status::Snapshot;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ViewInfo {
    pub id: u64,
    pub projection: u64,
    pub active: bool,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub shown: Shown,
    pub controls: bool,
    pub visible: bool,
}

impl Default for ViewInfo {
    fn default() -> Self {
        Self {
            id: 0,
            projection: 0,
            active: false,
            cols: None,
            rows: None,
            shown: Shown::default(),
            controls: false,
            visible: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Shown {
    pub project: Option<u64>,
    pub workspace: Option<u64>,
    pub tab: Option<u64>,
    pub pane: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ViewLine {
    #[serde(flatten)]
    pub snapshot: Snapshot,
    pub view: ViewInfo,
}

#[cfg(test)]
mod tests;
