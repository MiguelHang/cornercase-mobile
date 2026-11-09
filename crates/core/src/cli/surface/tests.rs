use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::cli::surface::fixtures::{FRAME, PATCH};

fn parsed(text: &str) -> Frame {
    serde_json::from_str(text).expect("a surface line")
}

fn read(line: &Value) -> Frame {
    Frame::deserialize(line).expect("a surface line")
}

fn pane(frame: &Frame, id: u64) -> &WirePane {
    frame.panes.iter().find(|pane| pane.pane == id).expect("the pane is in the frame")
}

fn all_lines(pane: &WirePane) -> &Vec<Vec<WireRun>> {
    match &pane.lines {
        Some(WireLines::All(lines)) => lines,
        other => panic!("expected every line, got {other:?}"),
    }
}

fn changed_lines(pane: &WirePane) -> &HashMap<String, Vec<WireRun>> {
    match &pane.lines {
        Some(WireLines::Changed(lines)) => lines,
        other => panic!("expected the changed lines, got {other:?}"),
    }
}

fn run(style: usize, hyperlink: Option<usize>, cells: &[&str]) -> WireRun {
    WireRun { style, hyperlink, cells: cells.iter().map(|cell| (*cell).to_owned()).collect() }
}

mod a_complete_frame {
    use super::*;

    #[test]
    fn reads_its_header() {
        let frame = parsed(FRAME);
        assert_eq!(
            (frame.boot.as_str(), frame.session, frame.revision, frame.surface, frame.tab),
            ("48211-1760695200123456789", 7, 3, 2, 12)
        );
        assert_eq!((frame.cols, frame.rows, frame.base, frame.projection, frame.view), (120, 40, None, None, None));
    }

    #[test]
    fn reads_the_split_between_its_two_panes() {
        let frame = parsed(FRAME);
        let branch = Branch { dir: "right".to_owned(), first: Node::Leaf(13), second: Node::Leaf(14) };
        assert_eq!(frame.layout, Some(Node::Split(Box::new(branch))));
        let panes: Vec<_> = frame.panes.iter().map(|pane| pane.pane).collect();
        assert_eq!(panes, vec![13, 14]);
    }

    #[test]
    fn reads_the_style_table() {
        let styles = parsed(FRAME).styles;
        assert_eq!(styles.len(), 5);
        assert_eq!(styles[0], Style::default());
        assert_eq!(styles[1], Style { fg: Some(Colour::Index(2)), bold: true, ..Style::default() });
        assert_eq!(styles[2], Style { bg: Some(Colour::Rgb([40, 44, 52])), inverse: true, ..Style::default() });
    }

    #[test]
    fn reads_underline_styles_colours_and_overline() {
        let styles = parsed(FRAME).styles;
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
        assert_eq!(styles[3], curly);
        assert_eq!(styles[4], overlined);
    }

    #[test]
    fn reads_the_hyperlink_table() {
        let uris: Vec<_> = parsed(FRAME).hyperlinks.into_iter().map(|link| link.uri).collect();
        assert_eq!(uris, vec!["https://example.com/docs/a-long-page", "file:///home/u/web-shop/README.md"]);
    }

    #[test]
    fn reads_the_runs_of_every_line() {
        let frame = parsed(FRAME);
        let lines = all_lines(pane(&frame, 13));
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[1], vec![run(1, None, &["o", "k"]), run(0, None, &[" ", "中", "", "x"])]);
        assert_eq!(lines[3], vec![run(0, Some(0), &["c", "s"]), run(4, Some(0), &["!"]), run(4, Some(1), &["?"])]);
        assert_eq!(lines[4], Vec::<WireRun>::new());
    }

    #[test]
    fn reads_a_pane_with_its_cursor_modes_and_scroll() {
        let frame = parsed(FRAME);
        let shown = pane(&frame, 13);
        assert_eq!((shown.active, shown.size), (Some(true), Some(Size { cols: 51, rows: 5 })));
        assert_eq!(shown.rect, Some(Area { x: 0, y: 0, cols: 59, rows: 40 }));
        assert_eq!(shown.cursor, Some(Cursor { x: 2, y: 1, shape: CursorShape::Bar, blinking: true }));
        assert_eq!(shown.scroll, Some(Scroll { total: 1200, offset: 1195, rows: 5 }));
        let modes = shown.modes.expect("the pane's modes");
        assert_eq!((modes.mouse, modes.mouse_encoding), (MouseMode::PressRelease, MouseEncoding::Sgr));
        assert!(modes.alternate_screen && modes.bracketed_paste && modes.focus_events && !modes.application_cursor);
        assert_eq!(modes.kitty_keyboard, 1);
    }

    #[test]
    fn reads_the_programs_colours_and_palette() {
        let frame = parsed(FRAME);
        let palette = BTreeMap::from([("1".to_owned(), [255, 0, 0]), ("200".to_owned(), [16, 32, 48])]);
        let colours =
            Colours { fg: Some([220, 220, 220]), bg: Some([16, 16, 16]), cursor: Some([255, 0, 255]), palette };
        assert_eq!(pane(&frame, 13).colours, Some(colours));
    }

    #[test]
    fn reads_a_hidden_cursor_and_no_colours() {
        let frame = parsed(FRAME);
        let hidden = pane(&frame, 14);
        assert_eq!((hidden.cursor, hidden.colours.as_ref(), hidden.copy), (None, None, None));
        let modes = hidden.modes.expect("the pane's modes");
        assert_eq!((modes.mouse, modes.mouse_encoding), (MouseMode::None, MouseEncoding::Default));
        assert_eq!(all_lines(hidden), &vec![vec![run(2, None, &["x"])]]);
    }
}

mod a_patch {
    use super::*;

    #[test]
    fn names_the_revision_it_applies_to() {
        let frame = parsed(PATCH);
        assert_eq!((frame.revision, frame.base, frame.surface, frame.session), (4, Some(3), 2, 7));
        assert_eq!(frame.layout, None);
    }

    #[test]
    fn reads_the_copy_and_the_changed_lines() {
        let frame = parsed(PATCH);
        let scrolled = pane(&frame, 13);
        assert_eq!(scrolled.copy, Some(Copied { from: 1, to: 0, count: 3 }));
        let lines = changed_lines(scrolled);
        let mut rows: Vec<_> = lines.keys().map(String::as_str).collect();
        rows.sort_unstable();
        assert_eq!(rows, vec!["3", "4"]);
        assert_eq!(lines["4"], vec![run(2, None, &["$", " "])]);
    }

    #[test]
    fn leaves_out_what_did_not_change() {
        let frame = parsed(PATCH);
        let scrolled = pane(&frame, 13);
        assert_eq!((scrolled.active, scrolled.modes, scrolled.colours.as_ref()), (Some(false), None, None));
        assert_eq!((scrolled.rect, scrolled.size), (None, None));
        assert_eq!(scrolled.cursor, Some(Cursor { x: 6, y: 4, shape: CursorShape::Bar, blinking: true }));
    }

    #[test]
    fn reads_empty_colours_as_present() {
        let frame = parsed(PATCH);
        let focused = pane(&frame, 14);
        assert_eq!(
            (focused.active, focused.colours.as_ref(), focused.lines.as_ref()),
            (Some(true), Some(&Colours::default()), None)
        );
        assert!(focused.modes.expect("the pane's modes").bracketed_paste);
    }
}

mod missing_and_unknown_keys {
    use super::*;

    #[rstest]
    #[case::nothing(json!({}))]
    #[case::nulls(json!({ "base": null, "projection": null, "layout": null, "view": null }))]
    #[case::only_unknown_keys(json!({ "dividers": [], "popup": { "pane": 1 }, "later": true }))]
    fn give_the_defaults(#[case] line: Value) {
        assert_eq!(read(&line), Frame::default());
    }

    #[rstest]
    #[case::a_pane_with_only_its_id(json!({ "pane": 9 }), WirePane { pane: 9, ..WirePane::default() })]
    #[case::a_pane_with_unknown_keys(
        json!({ "pane": 9, "later": 1, "modes": { "later": true } }),
        WirePane { pane: 9, modes: Some(Modes::default()), ..WirePane::default() }
    )]
    #[case::a_copy_without_a_count(
        json!({ "pane": 9, "copy": { "from": 2 } }),
        WirePane { pane: 9, copy: Some(Copied { from: 2, ..Copied::default() }), ..WirePane::default() }
    )]
    #[case::a_run_without_cells(
        json!({ "pane": 9, "lines": [[{ "hyperlink": 1 }]] }),
        WirePane { pane: 9, lines: Some(WireLines::All(vec![vec![run(0, Some(1), &[])]])), ..WirePane::default() }
    )]
    fn read_a_pane(#[case] pane: Value, #[case] expected: WirePane) {
        assert_eq!(read(&json!({ "panes": [pane] })).panes, vec![expected]);
    }

    #[test]
    fn read_a_view_and_its_projection() {
        let frame = read(&json!({ "view": 41, "projection": 3, "later": { "x": 1 } }));
        assert_eq!((frame.view, frame.projection), (Some(41), Some(3)));
    }

    #[test]
    fn read_a_style_with_unknown_attributes() {
        let frame = read(&json!({ "styles": [{ "bold": true, "sparkle": true }, { "fg": { "index": 7 } }] }));
        assert_eq!(frame.styles[0], Style { bold: true, ..Style::default() });
        assert_eq!(frame.styles[1].fg, Some(Colour::Index(7)));
    }

    #[test]
    fn read_a_split_without_its_direction() {
        let frame = read(&json!({ "layout": { "split": { "first": { "leaf": 1 }, "second": { "leaf": 2 } } } }));
        let branch = Branch { dir: String::new(), first: Node::Leaf(1), second: Node::Leaf(2) };
        assert_eq!(frame.layout, Some(Node::Split(Box::new(branch))));
    }

    #[test]
    fn read_later_values_of_the_closed_lists() {
        let pane = json!({
            "pane": 9,
            "cursor": { "x": 1, "shape": "later" },
            "modes": { "mouse": "later", "mouse_encoding": "later", "bracketed_paste": true }
        });
        let frame = read(&json!({ "styles": [{ "underline": true, "underline_style": "later" }], "panes": [pane] }));
        assert_eq!(frame.styles[0].underline_kind, Some(UnderlineKind::Other));
        let read_pane = &frame.panes[0];
        assert_eq!(read_pane.cursor, Some(Cursor { x: 1, ..Cursor::default() }));
        let modes = read_pane.modes.expect("the pane's modes");
        assert_eq!(
            (modes.mouse, modes.mouse_encoding, modes.bracketed_paste),
            (MouseMode::None, MouseEncoding::Default, true)
        );
    }

    #[test]
    fn refuse_lines_that_are_neither_a_list_nor_a_map() {
        let error =
            Frame::deserialize(&json!({ "panes": [{ "pane": 9, "lines": 3 }] })).expect_err("lines as a number");
        assert!(error.to_string().contains("a list of every line or a map of the changed rows"), "{error}");
    }
}
