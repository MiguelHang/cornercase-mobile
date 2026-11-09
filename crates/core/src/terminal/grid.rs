use std::sync::Arc;

use crate::cli::surface::{
    Area, Colours, Cursor, Frame, Hyperlink, Modes, Node, Scroll, Size, Style, WireLines, WirePane, WireRun,
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
}

impl Grid {
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
    }
}

#[cfg(test)]
mod tests;
