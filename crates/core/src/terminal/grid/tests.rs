use rstest::rstest;
use serde::Deserialize;
use serde_json::{Value, json};

use super::*;
use crate::cli::surface::fixtures::FRAME;
use crate::cli::surface::{Branch, Colour, CursorShape, UnderlineKind};

fn applied(line: &Value) -> Grid {
    let mut grid = Grid::default();
    grid.apply_frame(Frame::deserialize(line).expect("a surface line"));
    grid
}

fn from_fixture() -> Grid {
    applied(&serde_json::from_str(FRAME).expect("the frame fixture"))
}

fn pane(grid: &Grid, id: u64) -> &PaneGrid {
    grid.panes.iter().find(|pane| pane.pane == id).expect("the pane is in the grid")
}

fn run(text: &str, width: u16, style: Style, link: Option<&str>) -> Run {
    Run { text: text.to_owned(), width, style, link: link.map(Arc::from) }
}

const DOCS: &str = "https://example.com/docs/a-long-page";

mod a_complete_frame {
    use super::*;

    #[test]
    fn replaces_the_header() {
        let grid = from_fixture();
        assert_eq!(
            (grid.boot.as_str(), grid.surface, grid.revision, grid.tab, grid.projection),
            ("48211-1760695200123456789", 2, 3, 12, None)
        );
        assert_eq!((grid.cols, grid.rows), (120, 40));
        let branch = Branch { dir: "right".to_owned(), first: Node::Leaf(13), second: Node::Leaf(14) };
        assert_eq!(grid.layout, Some(Node::Split(Box::new(branch))));
    }

    #[test]
    fn keeps_each_pane_with_its_cursor_and_modes() {
        let grid = from_fixture();
        let ids: Vec<_> = grid.panes.iter().map(|pane| pane.pane).collect();
        assert_eq!(ids, vec![13, 14]);
        let shown = pane(&grid, 13);
        assert_eq!((shown.active, shown.size, shown.lines.len()), (true, Size { cols: 51, rows: 5 }, 5));
        assert_eq!(shown.cursor, Some(Cursor { x: 2, y: 1, shape: CursorShape::Bar, blinking: true }));
        assert_eq!(shown.scroll, Scroll { total: 1200, offset: 1195, rows: 5 });
        assert!(shown.modes.bracketed_paste);
        assert_eq!(shown.colours.bg, Some([16, 16, 16]));
    }

    #[test]
    fn hides_the_cursor_of_a_pane_without_one() {
        let grid = from_fixture();
        let hidden = pane(&grid, 14);
        assert_eq!((hidden.active, hidden.cursor, &hidden.colours), (false, None, &Colours::default()));
        let shaded = Style { bg: Some(Colour::Rgb([40, 44, 52])), inverse: true, ..Style::default() };
        assert_eq!(hidden.lines, vec![vec![run("x", 1, shaded, None)]]);
    }

    #[test]
    fn joins_the_cells_of_a_wide_character() {
        let grid = from_fixture();
        let bold = Style { fg: Some(Colour::Index(2)), bold: true, ..Style::default() };
        let expected = vec![run("ok", 2, bold, None), run(" 中x", 4, Style::default(), None)];
        assert_eq!(pane(&grid, 13).lines[1], expected);
    }

    #[test]
    fn resolves_styles_and_links() {
        let grid = from_fixture();
        let curly = Style {
            underline: true,
            underline_kind: Some(UnderlineKind::Curly),
            underline_colour: Some(Colour::Index(196)),
            ..Style::default()
        };
        let overlined = Style {
            underline: true,
            underline_colour: Some(Colour::Rgb([255, 128, 0])),
            overline: true,
            ..Style::default()
        };
        let lines = &pane(&grid, 13).lines;
        assert_eq!(lines[2], vec![run("err", 3, curly, None), run("see do", 6, Style::default(), Some(DOCS))]);
        let readme = Some("file:///home/u/web-shop/README.md");
        let expected = vec![run("cs", 2, Style::default(), Some(DOCS)), run("!", 1, overlined, Some(DOCS))];
        assert_eq!(lines[3][..2], expected);
        assert_eq!(lines[3][2], run("?", 1, overlined, readme));
    }

    #[test]
    fn shares_one_link_between_its_runs() {
        let grid = from_fixture();
        let lines = &pane(&grid, 13).lines;
        let first = lines[2][1].link.as_ref().expect("a link");
        let second = lines[3][0].link.as_ref().expect("a link");
        assert!(Arc::ptr_eq(first, second));
    }

    #[test]
    fn keeps_an_empty_line() {
        let grid = from_fixture();
        assert_eq!(pane(&grid, 13).lines[4], Line::new());
    }
}

mod a_later_frame {
    use super::*;

    #[test]
    fn replaces_everything() {
        let mut grid = applied(&json!({ "boot": "a", "projection": 3, "panes": [{ "pane": 1 }, { "pane": 2 }] }));
        let next = json!({
            "boot": "b", "revision": 9, "surface": 5, "tab": 4, "cols": 10, "rows": 2,
            "panes": [{ "pane": 7, "active": true, "size": { "cols": 10, "rows": 2 }, "lines": [[{ "cells": ["a"] }]] }]
        });
        grid.apply_frame(Frame::deserialize(&next).expect("a surface line"));
        let shown = PaneGrid {
            pane: 7,
            active: true,
            size: Size { cols: 10, rows: 2 },
            lines: vec![vec![run("a", 1, Style::default(), None)]],
            ..PaneGrid::default()
        };
        let expected = Grid {
            boot: "b".to_owned(),
            surface: 5,
            revision: 9,
            tab: 4,
            cols: 10,
            rows: 2,
            panes: vec![shown],
            ..Grid::default()
        };
        assert_eq!(grid, expected);
    }
}

mod the_rows_of_a_pane {
    use super::*;

    #[rstest]
    #[case::beyond_its_size(json!({ "size": { "rows": 2 }, "lines": [[], [], []] }), 2)]
    #[case::within_its_size(json!({ "size": { "rows": 4 }, "lines": [[], [], []] }), 3)]
    #[case::without_a_size(json!({ "lines": [[], []] }), 0)]
    #[case::changed_ones_only(json!({ "size": { "rows": 4 }, "lines": { "0": [] } }), 0)]
    #[case::none(json!({ "size": { "rows": 4 } }), 0)]
    fn stop_at_its_size(#[case] pane: Value, #[case] rows: usize) {
        let grid = applied(&json!({ "panes": [pane] }));
        assert_eq!(grid.panes[0].lines.len(), rows);
    }
}

mod indices_outside_the_tables {
    use super::*;

    #[rstest]
    #[case::a_known_style_and_link(json!({ "style": 1, "hyperlink": 0 }), Style { bold: true, ..Style::default() }, Some("a"))]
    #[case::an_unknown_style(json!({ "style": 2 }), Style::default(), None)]
    #[case::an_unknown_link(json!({ "style": 1, "hyperlink": 1 }), Style { bold: true, ..Style::default() }, None)]
    fn resolve_to_the_defaults(#[case] wire: Value, #[case] style: Style, #[case] link: Option<&str>) {
        let grid = applied(&json!({
            "styles": [{}, { "bold": true }],
            "hyperlinks": [{ "uri": "a" }],
            "panes": [{ "size": { "rows": 1 }, "lines": [[wire]] }]
        }));
        assert_eq!(grid.panes[0].lines, vec![vec![run("", 0, style, link)]]);
    }
}
