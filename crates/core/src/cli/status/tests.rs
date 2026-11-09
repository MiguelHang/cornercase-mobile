use serde_json::json;

use super::*;
use crate::cli::status::fixtures::{VIEW, WATCH};

fn parsed(text: &str) -> Snapshot {
    serde_json::from_str(text).expect("a status line")
}

fn agent(id: u64, title: Option<String>, parent: Option<u64>, active: bool) -> Pane {
    Pane {
        id,
        agent: Some("claude".into()),
        status: Some("working".into()),
        program: Some("claude".into()),
        title,
        path: Some("/home/me/web-shop".into()),
        parent,
        active,
        dialog: false,
        at_prompt: None,
    }
}

fn tab() -> Tab {
    Tab {
        id: 3,
        name: "agents".to_owned(),
        custom_name: Some("agents".into()),
        status: Some("working".into()),
        active: true,
        panes: vec![agent(4, Some("✳ Claude Code".into()), None, true), agent(5, None, Some(4), false)],
    }
}

fn expected() -> Snapshot {
    let workspace = Workspace {
        id: 2,
        name: "feat/dark-mode".to_owned(),
        custom_name: None,
        path: Some("/home/me/.cornercase/worktrees/web-shop/feat-dark-mode".into()),
        branch: Some("feat/dark-mode".into()),
        worktree: true,
        tabs: vec![tab()],
    };
    let project = Project {
        id: 1,
        name: "shop".to_owned(),
        custom_name: Some("shop".into()),
        path: Some("/home/me/web-shop".into()),
        group: Some(10),
        workspaces: vec![workspace],
    };
    let group = Group { id: 10, name: "work".to_owned(), icon: "●".to_owned(), colour: 33, collapsed: false };
    Snapshot {
        boot: "4242-1760600000".to_owned(),
        revision: 7,
        version: "0.12.111".to_owned(),
        groups: vec![group],
        projects: vec![project],
    }
}

mod reading {
    use super::*;

    #[test]
    fn reads_a_line_of_status_follow() {
        assert_eq!(parsed(WATCH), expected());
    }

    #[test]
    fn reads_a_line_of_view_as_the_same_session() {
        assert_eq!(parsed(VIEW), expected());
    }

    #[test]
    fn ignores_keys_it_does_not_know() {
        let line = json!({
            "boot": "1-2",
            "shown": { "project": 1 },
            "view": { "id": 41 },
            "machines": [{ "name": "vps" }],
            "projects": [{
                "id": 1,
                "accounts": {},
                "workspaces": [{ "id": 2, "post": { "url": "u" }, "tabs": [{
                    "id": 3,
                    "controller": 41,
                    "layout": { "leaf": 4 },
                    "panes": [{ "id": 4, "model": "Opus 5.5", "later": true }],
                }] }],
            }],
        });
        let snapshot = Snapshot::deserialize(line).expect("a status line");
        let pane = Pane { id: 4, ..Pane::default() };
        let tab = Tab { id: 3, panes: vec![pane], ..Tab::default() };
        let workspace = Workspace { id: 2, tabs: vec![tab], ..Workspace::default() };
        let project = Project { id: 1, workspaces: vec![workspace], ..Project::default() };
        let expected = Snapshot { boot: "1-2".to_owned(), projects: vec![project], ..Snapshot::default() };
        assert_eq!(snapshot, expected);
    }

    #[test]
    fn takes_a_pane_without_a_folder() {
        let line = json!({ "projects": [{ "path": null, "workspaces": [{ "path": null, "tabs": [{ "panes": [{
            "id": 4,
            "path": null,
            "at_prompt": true,
        }] }] }] }] });
        let snapshot = Snapshot::deserialize(line).expect("a status line");
        let pane = &snapshot.projects[0].workspaces[0].tabs[0].panes[0];
        assert_eq!(*pane, Pane { id: 4, at_prompt: Some(true), ..Pane::default() });
        assert_eq!(snapshot.projects[0].path, None);
    }

    #[test]
    fn leaves_what_is_missing_empty() {
        assert_eq!(parsed("{}"), Snapshot::default());
    }
}
