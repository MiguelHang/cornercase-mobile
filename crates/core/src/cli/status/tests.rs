use serde_json::json;

use super::*;
use crate::cli::status::fixtures::{STATUS, STATUS_TREE, VIEW, WATCH};

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
        role: None,
        collapsed: false,
        active,
        dialog: false,
        at_prompt: None,
    }
}

fn panes(snapshot: &Snapshot) -> impl Iterator<Item = &Pane> {
    snapshot
        .projects
        .iter()
        .flat_map(|project| &project.workspaces)
        .flat_map(|workspace| &workspace.tabs)
        .flat_map(|tab| &tab.panes)
}

fn tab() -> Tab {
    Tab {
        id: 3,
        name: "agents".to_owned(),
        custom_name: Some("agents".into()),
        status: Some("working".into()),
        active: true,
        panes: vec![
            agent(4, Some("✳ Claude Code".into()), None, true),
            Pane { role: Some("coder".into()), ..agent(5, None, Some(4), false) },
        ],
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

mod writing {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::status(STATUS)]
    #[case::status_follow(WATCH)]
    #[case::a_tree_of_agents(STATUS_TREE)]
    fn reads_back_the_same_session_it_writes(#[case] line: &str) {
        let snapshot = parsed(line);
        let written = serde_json::to_string(&snapshot).expect("JSON");
        assert_eq!(parsed(&written), snapshot);
    }

    #[test]
    fn writes_every_field_of_a_pane_with_the_names_of_the_contract() {
        let pane = Pane {
            role: Some("coder".into()),
            collapsed: true,
            at_prompt: Some(true),
            ..agent(5, None, Some(4), false)
        };
        let expected = json!({
            "id": 5,
            "agent": "claude",
            "status": "working",
            "program": "claude",
            "title": null,
            "path": "/home/me/web-shop",
            "parent": 4,
            "role": "coder",
            "collapsed": true,
            "active": false,
            "dialog": false,
            "at_prompt": true,
        });
        assert_eq!(serde_json::to_value(pane).expect("JSON"), expected);
    }
}

mod the_real_server {
    use super::*;

    #[test]
    fn reads_its_version_and_groups() {
        let snapshot = parsed(STATUS);
        let group = Group { id: 5, name: "work".to_owned(), icon: "●".to_owned(), colour: 12, collapsed: false };
        assert_eq!(snapshot.version, "0.13.19");
        assert_eq!(snapshot.boot, "62924-1794204195277384000");
        assert_eq!(snapshot.groups, vec![group]);
    }

    #[test]
    fn has_no_revision_outside_a_stream() {
        assert_eq!(parsed(STATUS).revision, 0);
    }

    #[test]
    fn reads_its_projects_and_their_groups() {
        let snapshot = parsed(STATUS);
        let projects: Vec<_> =
            snapshot.projects.iter().map(|project| (project.id, project.name.as_str(), project.group)).collect();
        assert_eq!(projects, vec![(4, "notes", None), (9, "web-shop", Some(5))]);
        assert_eq!(snapshot.projects[1].path.as_deref(), Some("/home/me/src/web-shop"));
    }

    #[test]
    fn reads_the_worktree_and_its_split_tab() {
        let snapshot = parsed(STATUS);
        let worktree = &snapshot.projects[1].workspaces[1];
        assert_eq!((worktree.id, worktree.branch.as_deref(), worktree.worktree), (10, Some("feat/dark-mode"), true));
        let tab = &worktree.tabs[1];
        assert_eq!(
            (tab.id, tab.custom_name.as_deref(), tab.active, tab.status.as_deref()),
            (14, Some("agents"), false, None)
        );
        let panes: Vec<_> = tab.panes.iter().map(|pane| (pane.id, pane.active, pane.at_prompt, pane.parent)).collect();
        assert_eq!(panes, vec![(13, true, Some(true), None), (15, false, Some(true), None)]);
    }

    #[test]
    fn reads_every_pane_as_a_shell() {
        let snapshot = parsed(STATUS);
        let panes: Vec<_> =
            panes(&snapshot).map(|pane| (pane.id, pane.program.as_deref(), pane.agent.as_deref())).collect();
        let shell = |id| (id, Some("bash"), None);
        assert_eq!(panes, vec![shell(1), shell(6), shell(11), shell(13), shell(15)]);
    }
}

mod a_tree_of_agents {
    use super::*;

    fn agents(snapshot: &Snapshot) -> Vec<(u64, Option<u64>, Option<&str>, bool)> {
        panes(snapshot)
            .filter(|pane| pane.agent.is_some())
            .map(|pane| (pane.id, pane.parent, pane.role.as_deref(), pane.collapsed))
            .collect()
    }

    #[test]
    fn reads_who_started_each_agent_its_role_and_its_fold() {
        let snapshot = parsed(STATUS_TREE);
        let expected = vec![
            (5, None, None, true),
            (7, Some(5), None, false),
            (9, Some(7), None, false),
            (12, Some(5), None, false),
            (16, None, Some("orchestrator"), false),
            (17, Some(16), Some("coder"), false),
            (18, Some(16), Some("reviewer"), false),
        ];
        assert_eq!(snapshot.version, "0.13.19");
        assert_eq!(agents(&snapshot), expected);
    }

    #[test]
    fn hangs_a_child_in_another_workspace_from_its_parent() {
        let snapshot = parsed(STATUS_TREE);
        let worktree = &snapshot.projects[0].workspaces[1];
        let pane = &worktree.tabs[0].panes[0];
        assert_eq!((worktree.branch.as_deref(), pane.id, pane.parent), (Some("feat/dark-mode"), 12, Some(5)));
    }

    #[test]
    fn reads_a_pane_without_a_role_or_a_fold_as_neither() {
        let snapshot = parsed(STATUS_TREE);
        let shell = &snapshot.projects[0].workspaces[0].tabs[0].panes[0];
        assert_eq!((shell.id, shell.role.as_deref(), shell.collapsed), (1, None, false));
    }
}
