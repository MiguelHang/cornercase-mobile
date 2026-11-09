use super::grid::{Changes, Grid, Line, PaneGrid};
use crate::cli::surface::{Colours, Cursor, Size};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub pane: u64,
    pub size: Size,
    pub cursor: Option<Cursor>,
    pub colours: Colours,
    pub replaced: bool,
    pub rows: Vec<(u16, Line)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shown {
    asked: Option<u64>,
    showing: Option<u64>,
}

impl Shown {
    #[must_use]
    pub fn new(asked: Option<u64>) -> Self {
        Self { asked, showing: None }
    }

    pub fn update(&mut self, grid: &Grid, changes: &Changes) -> Option<Update> {
        let pane = self.chosen(grid)?;
        let switched = self.showing.replace(pane.pane) != Some(pane.pane);
        let own = changes.rows.get(&pane.pane);
        let rows = (0..pane.size.rows)
            .filter(|row| switched || own.is_some_and(|rows| rows.contains(row)))
            .map(|row| (row, pane.lines.get(usize::from(row)).cloned().unwrap_or_default()))
            .collect();
        Some(Update {
            pane: pane.pane,
            size: pane.size,
            cursor: pane.cursor,
            colours: pane.colours.clone(),
            replaced: switched || changes.replaced,
            rows,
        })
    }

    fn chosen<'a>(&self, grid: &'a Grid) -> Option<&'a PaneGrid> {
        let asked = self.asked.and_then(|asked| grid.panes.iter().find(|pane| pane.pane == asked));
        asked.or_else(|| grid.panes.iter().find(|pane| pane.active)).or_else(|| grid.panes.first())
    }
}

#[cfg(test)]
mod tests;
