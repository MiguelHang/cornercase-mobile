use std::collections::BTreeMap;

use rstest::rstest;
use serde::Deserialize;
use serde_json::{Value, json};

use super::*;
use crate::cli::surface::fixtures::{FRAME, PATCH};
use crate::cli::surface::{CursorShape, Frame};

struct Screen {
    grid: Grid,
    shown: Shown,
}

impl Screen {
    fn asking(asked: Option<u64>) -> Self {
        Self { grid: Grid::default(), shown: Shown::new(asked) }
    }

    fn on(&mut self, line: &Value) -> Option<Update> {
        self.grid.apply(Frame::deserialize(line).expect("a surface line"));
        let changes = self.grid.changed_rows();
        self.shown.update(&self.grid, &changes)
    }

    fn shows(&mut self, line: &Value) -> Update {
        self.on(line).expect("a pane to show")
    }
}

fn fixture(json: &str) -> Value {
    serde_json::from_str(json).expect("a surface fixture")
}

fn frame(panes: &Value) -> Value {
    json!({ "boot": "b", "surface": 1, "revision": 1, "panes": panes })
}

fn patch(panes: &Value) -> Value {
    json!({ "boot": "b", "surface": 1, "revision": 2, "base": 1, "panes": panes })
}

fn two_panes() -> Value {
    frame(&json!([
        { "pane": 1, "size": { "rows": 2 }, "lines": [[{ "cells": ["a"] }], [{ "cells": ["b"] }]] },
        { "pane": 2, "active": true, "size": { "rows": 1 }, "lines": [[{ "cells": ["c"] }]] }
    ]))
}

fn texts(update: &Update) -> Vec<(u16, String)> {
    update.rows.iter().map(|(row, line)| (*row, line.iter().map(|run| run.text.as_str()).collect())).collect()
}

fn numbers(update: &Update) -> Vec<u16> {
    update.rows.iter().map(|(row, _)| *row).collect()
}

mod choosing {
    use super::*;

    #[test]
    fn shows_the_pane_asked_for() {
        let update = Screen::asking(Some(14)).shows(&fixture(FRAME));
        assert_eq!((update.pane, update.replaced, texts(&update)), (14, true, vec![(0, "x".to_owned())]));
    }

    #[rstest]
    #[case::when_none_was_asked(None)]
    #[case::when_the_one_asked_is_not_there(Some(99))]
    fn shows_the_active_pane(#[case] asked: Option<u64>) {
        let update = Screen::asking(asked).shows(&fixture(FRAME));
        assert_eq!((update.pane, numbers(&update)), (13, vec![0, 1, 2, 3, 4]));
    }

    #[test]
    fn shows_the_first_pane_when_none_is_active() {
        let line = frame(&json!([{ "pane": 5, "size": { "rows": 1 } }, { "pane": 6, "size": { "rows": 1 } }]));
        assert_eq!(Screen::asking(None).shows(&line).pane, 5);
    }

    #[test]
    fn shows_nothing_without_a_pane() {
        assert_eq!(Screen::asking(None).on(&frame(&json!([]))), None);
    }

    #[test]
    fn falls_back_to_the_active_pane_with_every_row_when_its_pane_leaves() {
        let mut screen = Screen::asking(Some(1));
        assert_eq!(screen.shows(&two_panes()).pane, 1);
        let left =
            frame(&json!([{ "pane": 2, "active": true, "size": { "rows": 1 }, "lines": [[{ "cells": ["c"] }]] }]));
        let update = screen.shows(&left);
        assert_eq!((update.pane, update.replaced, texts(&update)), (2, true, vec![(0, "c".to_owned())]));
    }

    #[test]
    fn follows_the_active_pane_with_every_row_when_it_moves() {
        let mut screen = Screen::asking(None);
        assert_eq!(screen.shows(&fixture(FRAME)).pane, 13);
        let update = screen.shows(&fixture(PATCH));
        assert_eq!((update.pane, update.replaced, texts(&update)), (14, true, vec![(0, "x".to_owned())]));
    }
}

mod rows {
    use super::*;

    #[test]
    fn hands_only_the_rows_a_patch_changed() {
        let mut screen = Screen::asking(Some(13));
        screen.shows(&fixture(FRAME));
        let mut line = fixture(PATCH);
        line["panes"][0]["copy"] = Value::Null;
        let update = screen.shows(&line);
        let changed = vec![(3, "ok see".to_owned()), (4, "$ ".to_owned())];
        assert_eq!((update.replaced, texts(&update)), (false, changed));
    }

    #[test]
    fn hands_none_when_only_another_pane_changed() {
        let mut screen = Screen::asking(Some(1));
        screen.shows(&two_panes());
        let update = screen.shows(&patch(&json!([{ "pane": 2, "lines": { "0": [{ "cells": ["d"] }] } }])));
        assert_eq!((update.pane, update.replaced, update.rows), (1, false, Vec::new()));
    }

    #[test]
    fn hands_every_row_again_after_another_complete_frame() {
        let mut screen = Screen::asking(Some(1));
        screen.shows(&two_panes());
        let update = screen.shows(&two_panes());
        assert_eq!((update.replaced, texts(&update)), (true, vec![(0, "a".to_owned()), (1, "b".to_owned())]));
    }

    #[test]
    fn hands_an_empty_line_for_a_row_the_frame_left_out() {
        let line = frame(&json!([{ "pane": 1, "size": { "rows": 3 }, "lines": [[{ "cells": ["a"] }]] }]));
        let update = Screen::asking(None).shows(&line);
        assert_eq!(texts(&update), [(0, "a".to_owned()), (1, String::new()), (2, String::new())]);
    }

    #[test]
    fn never_hands_a_row_past_the_size_of_the_pane() {
        let lines = json!([[{ "cells": ["a"] }], [{ "cells": ["b"] }], [{ "cells": ["c"] }]]);
        let update = Screen::asking(None).shows(&frame(&json!([{ "pane": 1, "size": { "rows": 2 }, "lines": lines }])));
        assert_eq!(numbers(&update), [0, 1]);
    }
}

mod pane_state {
    use super::*;

    #[test]
    fn carries_the_size_cursor_and_colours_of_the_pane() {
        let update = Screen::asking(Some(13)).shows(&fixture(FRAME));
        let palette = BTreeMap::from([("1".to_owned(), [255, 0, 0]), ("200".to_owned(), [16, 32, 48])]);
        let colours =
            Colours { fg: Some([220, 220, 220]), bg: Some([16, 16, 16]), cursor: Some([255, 0, 255]), palette };
        let cursor = Cursor { x: 2, y: 1, shape: CursorShape::Bar, blinking: true };
        assert_eq!((update.size, update.cursor, update.colours), (Size { cols: 51, rows: 5 }, Some(cursor), colours));
    }

    #[test]
    fn carries_a_cursor_that_went_away() {
        let mut screen = Screen::asking(Some(1));
        screen.shows(&frame(&json!([{ "pane": 1, "size": { "rows": 1 }, "cursor": { "x": 1, "y": 0 } }])));
        assert_eq!(screen.shows(&patch(&json!([{ "pane": 1, "cursor": null }]))).cursor, None);
    }
}
