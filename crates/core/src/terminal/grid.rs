use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use crate::cli::surface::{
    Area, Colours, Copy, Cursor, Frame, Hyperlink, Modes, Node, Scroll, Size, Style, WireLines, WirePane, WireRun,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub width: u16,
    pub style: Style,
    pub link: Option<Arc<str>>,
}

pub type Line = Vec<Run>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneGrid {
    pub pane: u64,
    pub active: bool,
    pub rect: Area,
    pub size: Size,
    pub cursor: Option<Cursor>,
    pub modes: Modes,
    pub scroll: Scroll,
    pub colours: Colours,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grid {
    pub boot: String,
    pub surface: u64,
    pub revision: u64,
    pub tab: u64,
    pub projection: Option<u64>,
    pub cols: u16,
    pub rows: u16,
    pub layout: Option<Node>,
    pub panes: Vec<PaneGrid>,
    changes: Changes,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    pub replaced: bool,
    pub rows: BTreeMap<u64, BTreeSet<u16>>,
}

impl Changes {
    fn mark(&mut self, pane: u64, rows: BTreeSet<u16>) {
        if !rows.is_empty() {
            self.rows.entry(pane).or_default().extend(rows);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    Applied,
    NeedRedraw,
}

struct Tables {
    styles: Vec<Style>,
    links: Vec<Arc<str>>,
}

impl Tables {
    fn new(styles: Vec<Style>, hyperlinks: Vec<Hyperlink>) -> Self {
        Self { styles, links: hyperlinks.into_iter().map(|link| Arc::from(link.uri)).collect() }
    }

    fn run(&self, wire: &WireRun) -> Run {
        Run {
            text: wire.cells.concat(),
            width: u16::try_from(wire.cells.len()).unwrap_or(u16::MAX),
            style: self.styles.get(wire.style).copied().unwrap_or_default(),
            link: wire.hyperlink.and_then(|index| self.links.get(index).cloned()),
        }
    }

    fn line(&self, runs: &[WireRun]) -> Line {
        runs.iter().map(|run| self.run(run)).collect()
    }

    fn lines(&self, wire: &[Vec<WireRun>], rows: u16) -> Vec<Line> {
        wire.iter().take(usize::from(rows)).map(|runs| self.line(runs)).collect()
    }
}

impl PaneGrid {
    fn from_wire(wire: WirePane, tables: &Tables) -> Self {
        let size = wire.size.unwrap_or_default();
        Self {
            pane: wire.pane,
            active: wire.active.unwrap_or(false),
            rect: wire.rect.unwrap_or_default(),
            size,
            cursor: wire.cursor,
            modes: wire.modes.unwrap_or_default(),
            scroll: wire.scroll.unwrap_or_default(),
            colours: wire.colours.unwrap_or_default(),
            lines: match wire.lines {
                Some(WireLines::All(all)) => tables.lines(&all, size.rows),
                _ => Vec::new(),
            },
        }
    }

    fn all_rows(&self) -> BTreeSet<u16> {
        (0..self.size.rows).collect()
    }

    fn patch(&mut self, wire: WirePane, tables: &Tables) -> BTreeSet<u16> {
        self.cursor = wire.cursor;
        self.active = wire.active.unwrap_or(self.active);
        self.modes = wire.modes.unwrap_or(self.modes);
        self.scroll = wire.scroll.unwrap_or(self.scroll);
        if let Some(colours) = wire.colours {
            self.colours = colours;
        }
        let mut rows = wire.copy.map(|copy| self.copy_lines(copy)).unwrap_or_default();
        match wire.lines {
            Some(WireLines::Changed(changed)) => rows.extend(self.replace_rows(&changed, tables)),
            Some(WireLines::All(all)) => {
                self.lines = tables.lines(&all, self.size.rows);
                rows.extend(self.all_rows());
            }
            None => {}
        }
        rows
    }

    fn copy_lines(&mut self, copy: Copy) -> BTreeSet<u16> {
        let held = self.lines.clone();
        let mut copied = BTreeSet::new();
        for i in 0..usize::from(copy.count) {
            let (from, to) = (usize::from(copy.from) + i, usize::from(copy.to) + i);
            if let (Some(source), Some(target)) = (held.get(from), self.lines.get_mut(to)) {
                target.clone_from(source);
                copied.extend(u16::try_from(to).ok());
            }
        }
        copied
    }

    fn replace_rows(&mut self, changed: &HashMap<String, Vec<WireRun>>, tables: &Tables) -> BTreeSet<u16> {
        let mut replaced = BTreeSet::new();
        for (key, runs) in changed {
            let Ok(row) = key.parse::<u16>() else { continue };
            if row >= self.size.rows {
                continue;
            }
            let index = usize::from(row);
            if index >= self.lines.len() {
                self.lines.resize(index + 1, Line::new());
            }
            self.lines[index] = tables.line(runs);
            replaced.insert(row);
        }
        replaced
    }
}

impl Grid {
    pub fn apply(&mut self, frame: Frame) -> Applied {
        if frame.base.is_some() {
            return self.apply_patch(frame);
        }
        self.apply_frame(frame);
        Applied::Applied
    }

    pub fn apply_patch(&mut self, frame: Frame) -> Applied {
        if frame.base != Some(self.revision) || frame.surface != self.surface || frame.boot != self.boot {
            return Applied::NeedRedraw;
        }
        let tables = Tables::new(frame.styles, frame.hyperlinks);
        for wire in frame.panes {
            let Some(grid) = self.panes.iter_mut().find(|grid| grid.pane == wire.pane) else {
                return Applied::NeedRedraw;
            };
            let rows = grid.patch(wire, &tables);
            self.changes.mark(grid.pane, rows);
        }
        self.revision = frame.revision;
        self.projection = frame.projection.or(self.projection);
        Applied::Applied
    }

    pub fn apply_frame(&mut self, frame: Frame) {
        let tables = Tables::new(frame.styles, frame.hyperlinks);
        self.boot = frame.boot;
        self.surface = frame.surface;
        self.revision = frame.revision;
        self.tab = frame.tab;
        self.projection = frame.projection;
        self.cols = frame.cols;
        self.rows = frame.rows;
        self.layout = frame.layout;
        self.panes = frame.panes.into_iter().map(|pane| PaneGrid::from_wire(pane, &tables)).collect();
        self.changes = Changes { replaced: true, ..Changes::default() };
        for pane in &self.panes {
            self.changes.mark(pane.pane, pane.all_rows());
        }
    }

    pub fn changed_rows(&mut self) -> Changes {
        std::mem::take(&mut self.changes)
    }
}

#[cfg(test)]
mod tests;
