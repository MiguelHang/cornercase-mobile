use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::cli::status::fixtures::VIEW;

fn parsed(text: &str) -> ViewLine {
    serde_json::from_str(text).expect("a view line")
}

fn read(line: &Value) -> ViewLine {
    ViewLine::deserialize(line).expect("a view line")
}

mod a_view_line {
    use super::*;

    #[test]
    fn reads_the_view() {
        let shown = Shown { project: Some(1), workspace: Some(2), tab: Some(3), pane: Some(4) };
        let view = ViewInfo {
            id: 41,
            projection: 3,
            active: true,
            cols: Some(120),
            rows: Some(40),
            shown,
            controls: true,
            visible: true,
        };
        assert_eq!(parsed(VIEW).view, view);
    }

    #[test]
    fn keeps_the_session_it_carries() {
        let session: Snapshot = serde_json::from_str(VIEW).expect("a status line");
        let line = parsed(VIEW);
        assert_eq!((line.snapshot.revision, line.snapshot.projects.len()), (7, 1));
        assert_eq!(line.snapshot, session);
    }
}

mod missing_and_unknown_keys {
    use super::*;

    #[rstest]
    #[case::no_view(json!({}), true)]
    #[case::a_view_without_visible(json!({ "view": { "id": 41 } }), true)]
    #[case::a_hidden_view(json!({ "view": { "id": 41, "visible": false } }), false)]
    #[case::a_shown_view(json!({ "view": { "visible": true, "later": 1 } }), true)]
    fn read_whether_the_view_is_visible(#[case] line: Value, #[case] visible: bool) {
        assert_eq!(read(&line).view.visible, visible);
    }

    #[test]
    fn give_the_defaults() {
        let line = read(&json!({ "view": { "shown": { "tab": 3, "later": true } }, "later": [] }));
        let view = ViewInfo { shown: Shown { tab: Some(3), ..Shown::default() }, ..ViewInfo::default() };
        assert_eq!(line, ViewLine { snapshot: Snapshot::default(), view });
    }
}
