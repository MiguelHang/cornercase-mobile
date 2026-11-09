use rstest::rstest;
use serde::Deserialize;
use serde_json::{Value, json};

use super::*;
use crate::cli::surface::fixtures::{FRAME, PATCH};
use crate::cli::surface::{Branch, Colour, CursorShape, UnderlineKind};

fn applied(line: &Value) -> Grid {
    let mut grid = Grid::default();
    grid.apply_frame(Frame::deserialize(line).expect("a surface line"));
    grid
}

fn from_fixture() -> Grid {
    applied(&serde_json::from_str(FRAME).expect("the frame fixture"))
}

fn line(text: &str) -> Value {
    json!([{ "cells": text.chars().map(String::from).collect::<Vec<_>>() }])
}

fn lettered() -> Grid {
    applied(&json!({
        "boot": "b", "surface": 1, "revision": 5, "projection": 2,
        "panes": [
            { "pane": 1, "size": { "rows": 4 }, "lines": [line("a"), line("b"), line("c"), line("d")] },
            { "pane": 2, "size": { "rows": 3 } }
        ]
    }))
}

fn on_lettered(panes: &Value) -> Frame {
    Frame::deserialize(&json!({ "boot": "b", "surface": 1, "revision": 6, "base": 5, "panes": panes }))
        .expect("a surface line")
}

fn rows(grid: &Grid, id: u64) -> Vec<String> {
    pane(grid, id).lines.iter().map(|line| line.iter().map(|run| run.text.as_str()).collect()).collect()
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

mod the_patch_fixture {
    use super::*;

    fn patched() -> (Grid, Grid) {
        let before = from_fixture();
        let mut grid = before.clone();
        let applied = grid.apply_patch(serde_json::from_str(PATCH).expect("the patch fixture"));
        assert_eq!(applied, Applied::Applied);
        (before, grid)
    }

    #[test]
    fn moves_three_rows_up_and_replaces_the_last_two() {
        let (before, grid) = patched();
        let lines = &pane(&grid, 13).lines;
        assert_eq!(lines[..3], pane(&before, 13).lines[1..4]);
        let bold = Style { fg: Some(Colour::Index(2)), bold: true, ..Style::default() };
        assert_eq!(lines[3], vec![run("ok", 2, bold, None), run(" see", 4, Style::default(), Some(DOCS))]);
        let curly = Style { underline: true, underline_kind: Some(UnderlineKind::Curly), ..Style::default() };
        assert_eq!(lines[4], vec![run("$ ", 2, curly, None)]);
    }

    #[test]
    fn moves_the_cursor_and_the_scroll_of_the_first_pane() {
        let (before, grid) = patched();
        let scrolled = pane(&grid, 13);
        assert_eq!(scrolled.cursor, Some(Cursor { x: 6, y: 4, shape: CursorShape::Bar, blinking: true }));
        assert_eq!((scrolled.active, scrolled.scroll), (false, Scroll { total: 1201, offset: 1196, rows: 5 }));
        let kept = pane(&before, 13);
        assert_eq!((scrolled.modes, &scrolled.colours, scrolled.size), (kept.modes, &kept.colours, kept.size));
    }

    #[test]
    fn focuses_the_second_pane() {
        let (before, grid) = patched();
        let focused = pane(&grid, 14);
        assert!(focused.active && focused.modes.bracketed_paste);
        assert_eq!((focused.cursor, &focused.colours), (None, &Colours::default()));
        assert_eq!(focused.lines, pane(&before, 14).lines);
    }

    #[test]
    fn moves_to_its_revision() {
        let (before, grid) = patched();
        assert_eq!((grid.revision, grid.projection), (4, None));
        assert_eq!((&grid.layout, grid.cols, grid.rows), (&before.layout, before.cols, before.rows));
    }
}

mod a_patch {
    use super::*;

    #[test]
    fn hides_the_cursor_and_keeps_what_it_leaves_out() {
        let mut grid = from_fixture();
        let line = json!({ "boot": grid.boot, "surface": 2, "revision": 4, "base": 3, "panes": [{ "pane": 13 }] });
        assert_eq!(grid.apply_patch(Frame::deserialize(&line).expect("a surface line")), Applied::Applied);
        let mut expected = from_fixture();
        expected.revision = 4;
        expected.panes[0].cursor = None;
        assert_eq!(grid, expected);
    }

    #[test]
    fn empties_the_programs_colours() {
        let mut grid = from_fixture();
        let line = json!({ "boot": grid.boot, "surface": 2, "base": 3, "panes": [{ "pane": 13, "colours": {} }] });
        grid.apply_patch(Frame::deserialize(&line).expect("a surface line"));
        assert_eq!(pane(&grid, 13).colours, Colours::default());
    }

    #[rstest]
    #[case::without_one(json!({}), Some(2))]
    #[case::with_a_new_one(json!({ "projection": 7 }), Some(7))]
    fn keeps_the_last_projection(#[case] extra: Value, #[case] projection: Option<u64>) {
        let mut grid = lettered();
        let mut line = json!({ "boot": "b", "surface": 1, "revision": 6, "base": 5 });
        line.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
        assert_eq!(grid.apply_patch(Frame::deserialize(&line).expect("a surface line")), Applied::Applied);
        assert_eq!((grid.revision, grid.projection), (6, projection));
    }

    #[test]
    fn replaces_every_row_with_a_list_up_to_the_size() {
        let mut grid = lettered();
        grid.apply_patch(on_lettered(
            &json!([{ "pane": 1, "lines": [line("v"), line("w"), line("x"), line("y"), line("z")] }]),
        ));
        assert_eq!(rows(&grid, 1), vec!["v", "w", "x", "y"]);
    }
}

mod a_patch_for_another_grid {
    use super::*;

    #[rstest]
    #[case::another_base(json!({ "boot": "b", "surface": 1, "base": 4 }))]
    #[case::another_surface(json!({ "boot": "b", "surface": 2, "base": 5 }))]
    #[case::another_boot(json!({ "boot": "c", "surface": 1, "base": 5 }))]
    #[case::no_base(json!({ "boot": "b", "surface": 1 }))]
    fn needs_a_redraw_and_leaves_the_grid_alone(#[case] mut wire: Value) {
        let mut grid = lettered();
        wire["revision"] = json!(6);
        wire["panes"] = json!([{ "pane": 1, "lines": { "0": line("z") } }]);
        assert_eq!(grid.apply_patch(Frame::deserialize(&wire).expect("a surface line")), Applied::NeedRedraw);
        assert_eq!(grid, lettered());
    }

    #[test]
    fn needs_a_redraw_for_an_unknown_pane_after_patching_the_known_ones() {
        let mut grid = lettered();
        let panes = json!([{ "pane": 1, "active": true }, { "pane": 9 }, { "pane": 2, "active": true }]);
        assert_eq!(grid.apply_patch(on_lettered(&panes)), Applied::NeedRedraw);
        assert_eq!((pane(&grid, 1).active, pane(&grid, 2).active, grid.revision), (true, false, 5));
    }
}

mod a_copy {
    use super::*;

    #[rstest]
    #[case::up(1, 0, 3, &["b", "c", "d", "d"])]
    #[case::down(0, 1, 3, &["a", "a", "b", "c"])]
    #[case::from_beyond_the_rows(2, 0, 9, &["c", "d", "c", "d"])]
    #[case::to_beyond_the_rows(0, 3, 5, &["a", "b", "c", "a"])]
    #[case::entirely_outside(7, 9, 2, &["a", "b", "c", "d"])]
    #[case::of_the_largest_count(u16::MAX, u16::MAX, u16::MAX, &["a", "b", "c", "d"])]
    fn copies_the_rows_it_had(#[case] from: u16, #[case] to: u16, #[case] count: u16, #[case] expected: &[&str]) {
        let mut grid = lettered();
        let copy = json!({ "from": from, "to": to, "count": count });
        assert_eq!(grid.apply_patch(on_lettered(&json!([{ "pane": 1, "copy": copy }]))), Applied::Applied);
        assert_eq!(rows(&grid, 1), expected);
    }

    #[test]
    fn comes_before_the_changed_rows() {
        let mut grid = lettered();
        let wire = json!({ "pane": 1, "copy": { "from": 1, "to": 0, "count": 3 }, "lines": { "3": line("e") } });
        grid.apply_patch(on_lettered(&json!([wire])));
        assert_eq!(rows(&grid, 1), vec!["b", "c", "d", "e"]);
    }
}

mod the_changed_rows {
    use super::*;

    #[rstest]
    #[case::inside_the_rows(json!({ "1": line("z") }), &["a", "z", "c", "d"])]
    #[case::beyond_the_size(json!({ "4": line("z"), "70000": line("z") }), &["a", "b", "c", "d"])]
    #[case::with_a_key_that_is_no_number(json!({ "x": line("z"), "-1": line("z"), "2": line("z") }), &["a", "b", "z", "d"])]
    fn replace_their_rows(#[case] lines: Value, #[case] expected: &[&str]) {
        let mut grid = lettered();
        assert_eq!(grid.apply_patch(on_lettered(&json!([{ "pane": 1, "lines": lines }]))), Applied::Applied);
        assert_eq!(rows(&grid, 1), expected);
    }

    #[test]
    fn grow_the_pane_up_to_its_size() {
        let mut grid = lettered();
        grid.apply_patch(on_lettered(&json!([{ "pane": 2, "lines": { "1": line("y"), "3": line("z") } }])));
        assert_eq!(rows(&grid, 2), vec!["", "y"]);
    }
}

mod a_line_of_the_stream {
    use super::*;

    #[test]
    fn without_a_base_replaces_the_grid() {
        let mut grid = lettered();
        assert_eq!(grid.apply(serde_json::from_str(FRAME).expect("the frame fixture")), Applied::Applied);
        assert_eq!(grid, from_fixture());
    }

    #[rstest]
    #[case::that_fits(5, Applied::Applied, &["z", "b", "c", "d"])]
    #[case::that_does_not(4, Applied::NeedRedraw, &["a", "b", "c", "d"])]
    fn with_a_base_patches_it(#[case] base: u64, #[case] outcome: Applied, #[case] expected: &[&str]) {
        let mut grid = lettered();
        let wire =
            json!({ "boot": "b", "surface": 1, "base": base, "panes": [{ "pane": 1, "lines": { "0": line("z") } }] });
        assert_eq!(grid.apply(Frame::deserialize(&wire).expect("a surface line")), outcome);
        assert_eq!(rows(&grid, 1), expected);
    }
}
