use rstest::rstest;

use super::fixtures::{agent, pane, project, session, snapshot, tab, workspace};
use super::*;
use crate::cli::status::fixtures::{STATUS, WATCH};

const EARLIER: &str = "2026-11-09T12:00:00.000Z";
const LATER: &str = "2026-11-09T12:05:00.000Z";

fn parsed(text: &str) -> Snapshot {
    serde_json::from_str(text).expect("a status line")
}

fn order(snapshot: &Snapshot, since: &Since) -> Vec<u64> {
    agents(snapshot, since).iter().map(|row| row.pane).collect()
}

fn only_row(snapshot: &Snapshot) -> AgentRow {
    let rows = agents(snapshot, &Since::default());
    assert_eq!(rows.len(), 1);
    rows[0].clone()
}

fn seen(panes: Vec<Pane>) -> Since {
    let mut since = Since::default();
    since.reconcile(&snapshot(panes));
    since
}

fn seen_at(pane: u64, status: &str, time: &str) -> Since {
    let mut since = seen(vec![agent(pane, status)]);
    since.heard(pane, Some(status), time);
    since
}

mod an_event {
    use super::*;

    #[test]
    fn keeps_its_time_when_the_snapshot_then_shows_the_same_status() {
        let mut since = Since::default();
        since.heard(4, Some("waiting"), EARLIER);
        since.reconcile(&snapshot(vec![agent(4, "waiting")]));
        assert_eq!(since.of(4, Some("waiting")), Some(EARLIER));
    }

    #[test]
    fn gives_its_time_to_a_status_the_snapshot_showed_first() {
        let mut since = seen(vec![agent(4, "waiting")]);
        assert_eq!(since.of(4, Some("waiting")), None);
        since.heard(4, Some("waiting"), EARLIER);
        assert_eq!(since.of(4, Some("waiting")), Some(EARLIER));
    }

    #[test]
    fn keeps_its_time_when_a_later_snapshot_confirms_a_change_from_the_one_seen() {
        let mut since = seen(vec![agent(4, "working")]);
        since.heard(4, Some("waiting"), EARLIER);
        since.reconcile(&snapshot(vec![agent(4, "waiting")]));
        assert_eq!(since.of(4, Some("waiting")), Some(EARLIER));
    }

    #[test]
    fn replaces_the_time_of_an_earlier_event() {
        let mut since = Since::default();
        since.heard(4, Some("working"), EARLIER);
        since.heard(4, Some("waiting"), LATER);
        since.reconcile(&snapshot(vec![agent(4, "waiting")]));
        assert_eq!(since.of(4, Some("waiting")), Some(LATER));
    }

    #[test]
    fn replaced_gives_no_time_to_the_status_it_said() {
        let mut since = Since::default();
        since.heard(4, Some("working"), EARLIER);
        since.heard(4, Some("waiting"), LATER);
        since.reconcile(&snapshot(vec![agent(4, "working")]));
        assert_eq!(since.of(4, Some("working")), None);
    }

    #[test]
    fn of_a_pane_whose_agent_the_snapshot_does_not_show_yet_is_kept() {
        let mut since = Since::default();
        since.heard(4, Some("working"), EARLIER);
        since.reconcile(&snapshot(vec![pane(4, None, None)]));
        since.reconcile(&snapshot(vec![agent(4, "working")]));
        assert_eq!(since.of(4, Some("working")), Some(EARLIER));
    }

    #[test]
    fn can_say_a_pane_has_no_status() {
        let mut since = Since::default();
        since.heard(4, None, EARLIER);
        since.reconcile(&snapshot(vec![pane(4, Some("claude"), None)]));
        assert_eq!(since.of(4, None), Some(EARLIER));
    }
}

mod an_event_ahead_of_the_snapshot {
    use super::*;

    fn ahead() -> Since {
        let mut since = seen(vec![agent(4, "working")]);
        since.heard(4, Some("working"), EARLIER);
        since.heard(4, Some("waiting"), LATER);
        since
    }

    #[test]
    fn leaves_the_time_of_the_status_the_snapshot_still_shows() {
        assert_eq!(ahead().of(4, Some("working")), Some(EARLIER));
    }

    #[test]
    fn leaves_the_rows_as_they_were() {
        let snapshot = snapshot(vec![agent(4, "working")]);
        assert_eq!(agents(&snapshot, &ahead()), agents(&snapshot, &seen_at(4, "working", EARLIER)));
    }

    #[test]
    fn outlives_a_late_snapshot_of_the_status_before() {
        let mut since = ahead();
        since.reconcile(&snapshot(vec![agent(4, "working")]));
        assert_eq!(since.of(4, Some("working")), Some(EARLIER));
        since.reconcile(&snapshot(vec![agent(4, "waiting")]));
        assert_eq!(since.of(4, Some("waiting")), Some(LATER));
    }

    #[test]
    fn outlives_a_late_snapshot_of_a_status_in_between() {
        let mut since = seen(vec![agent(4, "working")]);
        since.heard(4, Some("waiting"), EARLIER);
        since.heard(4, Some("done"), LATER);
        since.reconcile(&snapshot(vec![agent(4, "waiting")]));
        assert_eq!(since.of(4, Some("waiting")), None);
        since.reconcile(&snapshot(vec![agent(4, "done")]));
        assert_eq!(since.of(4, Some("done")), Some(LATER));
    }

    #[test]
    fn is_dropped_by_an_event_back_to_the_status_seen() {
        let mut since = ahead();
        since.heard(4, Some("working"), LATER);
        assert_eq!(since.of(4, Some("working")), Some(LATER));
        since.reconcile(&snapshot(vec![agent(4, "waiting")]));
        assert_eq!(since.of(4, Some("waiting")), None);
    }
}

mod doubting {
    use super::*;

    #[test]
    fn forgets_every_time_and_keeps_every_status() {
        let mut since = seen_at(4, "waiting", EARLIER);
        assert!(since.doubt());
        assert_eq!(since.of(4, Some("waiting")), None);
        assert_eq!(since, seen(vec![agent(4, "waiting")]));
    }

    #[test]
    fn forgets_an_event_ahead_of_the_snapshot() {
        let mut since = seen(vec![agent(4, "working")]);
        since.heard(4, Some("waiting"), EARLIER);
        assert!(since.doubt());
        since.reconcile(&snapshot(vec![agent(4, "waiting")]));
        assert_eq!(since.of(4, Some("waiting")), None);
    }

    #[test]
    fn says_so_only_when_there_was_a_time_to_forget() {
        let mut since = seen(vec![agent(4, "waiting")]);
        assert!(!since.doubt());
    }

    #[test]
    fn says_so_when_a_pane_has_both_a_time_and_an_event_ahead() {
        let mut since = seen_at(4, "working", EARLIER);
        since.heard(4, Some("waiting"), LATER);
        assert!(since.doubt());
        assert!(!since.doubt());
    }

    #[test]
    fn takes_the_time_of_an_event_heard_after() {
        let mut since = seen_at(4, "waiting", EARLIER);
        since.doubt();
        since.heard(4, Some("waiting"), LATER);
        assert_eq!(since.of(4, Some("waiting")), Some(LATER));
    }
}

mod a_snapshot {
    use super::*;

    #[test]
    fn drops_the_time_of_a_status_it_does_not_show() {
        let mut since = Since::default();
        since.heard(4, Some("waiting"), EARLIER);
        since.reconcile(&snapshot(vec![agent(4, "done")]));
        assert_eq!(since.of(4, Some("done")), None);
        assert_eq!(since.of(4, Some("waiting")), None);
    }

    #[test]
    fn forgets_a_pane_it_no_longer_shows() {
        let mut since = Since::default();
        since.heard(4, Some("waiting"), EARLIER);
        since.reconcile(&snapshot(vec![agent(5, "working")]));
        assert_eq!(since, seen(vec![agent(5, "working")]));
        since.reconcile(&snapshot(vec![agent(4, "waiting")]));
        assert_eq!(since.of(4, Some("waiting")), None);
    }

    #[test]
    fn leaves_the_other_panes_as_they_were() {
        let mut since = Since::default();
        since.heard(4, Some("waiting"), EARLIER);
        since.heard(5, Some("working"), LATER);
        since.reconcile(&snapshot(vec![agent(4, "waiting"), agent(5, "done")]));
        assert_eq!(since.of(4, Some("waiting")), Some(EARLIER));
        assert_eq!(since.of(5, Some("done")), None);
    }
}

mod clearing {
    use super::*;

    #[test]
    fn forgets_every_pane() {
        let mut since = Since::default();
        since.heard(4, Some("waiting"), EARLIER);
        since.heard(5, Some("working"), LATER);
        since.clear();
        assert_eq!(since, Since::default());
    }
}

mod asking {
    use super::*;

    #[test]
    fn gives_no_time_for_another_status() {
        let mut since = Since::default();
        since.heard(4, Some("waiting"), EARLIER);
        assert_eq!(since.of(4, Some("working")), None);
        assert_eq!(since.of(4, None), None);
    }

    #[test]
    fn gives_no_time_for_a_pane_it_never_heard_of() {
        assert_eq!(Since::default().of(4, Some("waiting")), None);
    }
}

mod a_title {
    use super::*;

    #[rstest]
    #[case::after_a_mark(Some("✳ Claude Code"), Some("Claude Code"))]
    #[case::after_a_spinner_frame(Some("⠋ Fix login"), Some("Fix login"))]
    #[case::without_trailing_spaces(Some("Fix login  "), Some("Fix login"))]
    #[case::of_only_a_mark(Some("✳"), None)]
    #[case::of_only_spaces(Some("   "), None)]
    #[case::that_is_missing(None, None)]
    fn is_the_pane_title_from_its_first_letter(#[case] title: Option<&str>, #[case] expected: Option<&str>) {
        assert_eq!(agent_title(title).as_deref(), expected);
    }
}

mod a_row {
    use super::*;

    #[test]
    fn shows_the_names_given_over_the_names() {
        let mut snapshot = snapshot(vec![agent(4, "working")]);
        let project = &mut snapshot.projects[0];
        project.custom_name = Some("shop".to_owned());
        project.workspaces[0].custom_name = Some("dark mode".to_owned());
        project.workspaces[0].tabs[0].custom_name = Some("agents".to_owned());
        let row = only_row(&snapshot);
        assert_eq!((row.project_name.as_str(), row.workspace_name.as_str()), ("shop", "dark mode"));
        assert_eq!(row.tab_name, "agents");
    }

    #[test]
    fn shows_the_names_when_none_was_given() {
        let row = only_row(&snapshot(vec![agent(4, "working")]));
        assert_eq!((row.project_name.as_str(), row.workspace_name.as_str()), ("project 1", "workspace 2"));
        assert_eq!(row.tab_name, "tab 3");
    }

    #[test]
    fn counts_the_workspaces_of_its_project() {
        let busy = workspace(2, vec![tab(3, vec![agent(4, "working")])]);
        let snapshot = session(vec![project(1, vec![busy, workspace(5, vec![])])]);
        assert_eq!(only_row(&snapshot).workspace_count, 2);
    }

    #[test]
    fn carries_the_time_its_status_was_heard() {
        let snapshot = snapshot(vec![agent(4, "waiting")]);
        let mut since = Since::default();
        since.heard(4, Some("waiting"), EARLIER);
        since.reconcile(&snapshot);
        assert_eq!(agents(&snapshot, &since)[0].since.as_deref(), Some(EARLIER));
    }
}

mod the_agents {
    use super::*;

    fn watched(pane: u64, title: Option<&str>) -> AgentRow {
        AgentRow {
            project: 1,
            workspace: 2,
            tab: 3,
            pane,
            project_name: "shop".to_owned(),
            workspace_name: "feat/dark-mode".to_owned(),
            tab_name: "agents".to_owned(),
            workspace_count: 1,
            agent: "claude".to_owned(),
            title: title.map(str::to_owned),
            status: Some("working".to_owned()),
            since: None,
        }
    }

    #[test]
    fn of_a_watched_session_are_a_row_per_agent() {
        let rows = agents(&parsed(WATCH), &Since::default());
        assert_eq!(rows, vec![watched(4, Some("Claude Code")), watched(5, None)]);
    }

    #[test]
    fn of_a_session_without_agents_are_none() {
        assert_eq!(agents(&parsed(STATUS), &Since::default()), vec![]);
    }

    #[test]
    fn leave_out_the_panes_without_an_agent() {
        let snapshot = snapshot(vec![pane(4, None, Some("working")), agent(5, "idle"), pane(6, None, None)]);
        assert_eq!(order(&snapshot, &Since::default()), vec![5]);
    }

    #[test]
    fn come_from_every_project() {
        let first = project(1, vec![workspace(2, vec![tab(3, vec![agent(4, "idle")])])]);
        let second = project(5, vec![workspace(6, vec![tab(7, vec![agent(8, "idle")])])]);
        let rows = agents(&session(vec![first, second]), &Since::default());
        assert_eq!(rows.iter().map(|row| (row.project, row.pane)).collect::<Vec<_>>(), vec![(1, 4), (5, 8)]);
    }
}

mod the_order {
    use super::*;

    #[rstest]
    #[case::waiting_over_failed(Some("waiting"), Some("failed"))]
    #[case::failed_over_done(Some("failed"), Some("done"))]
    #[case::done_over_pending(Some("done"), Some("pending"))]
    #[case::pending_over_working(Some("pending"), Some("working"))]
    #[case::pending_over_shell(Some("pending"), Some("shell"))]
    #[case::working_over_idle(Some("working"), Some("idle"))]
    #[case::shell_over_no_status(Some("shell"), None)]
    #[case::working_over_an_unknown_status(Some("working"), Some("asleep"))]
    fn puts_the_louder_status_first(#[case] louder: Option<&str>, #[case] quieter: Option<&str>) {
        let snapshot = snapshot(vec![pane(4, Some("claude"), quieter), pane(5, Some("claude"), louder)]);
        assert_eq!(order(&snapshot, &Since::default()), vec![5, 4]);
    }

    #[rstest]
    #[case::working_and_shell(Some("working"), Some("shell"))]
    #[case::idle_and_no_status(Some("idle"), None)]
    #[case::idle_and_an_unknown_status(Some("idle"), Some("asleep"))]
    fn keeps_the_session_order_between_statuses_that_rank_alike(
        #[case] one: Option<&str>,
        #[case] other: Option<&str>,
    ) {
        let forth = snapshot(vec![pane(4, Some("claude"), one), pane(5, Some("claude"), other)]);
        let back = snapshot(vec![pane(4, Some("claude"), other), pane(5, Some("claude"), one)]);
        assert_eq!(order(&forth, &Since::default()), vec![4, 5]);
        assert_eq!(order(&back, &Since::default()), vec![4, 5]);
    }

    #[test]
    fn puts_the_longest_in_its_status_first() {
        let snapshot = snapshot(vec![agent(4, "waiting"), agent(5, "waiting")]);
        let mut since = seen(vec![agent(4, "waiting"), agent(5, "waiting")]);
        since.heard(4, Some("waiting"), LATER);
        since.heard(5, Some("waiting"), EARLIER);
        assert_eq!(order(&snapshot, &since), vec![5, 4]);
    }

    #[test]
    fn puts_an_unknown_time_before_a_known_one() {
        let snapshot = snapshot(vec![agent(4, "waiting"), agent(5, "waiting")]);
        let mut since = seen(vec![agent(4, "waiting"), agent(5, "waiting")]);
        since.heard(4, Some("waiting"), EARLIER);
        assert_eq!(order(&snapshot, &since), vec![5, 4]);
    }

    #[test]
    fn ranks_the_status_before_the_time() {
        let snapshot = snapshot(vec![agent(4, "working"), agent(5, "waiting")]);
        let mut since = seen(vec![agent(4, "working"), agent(5, "waiting")]);
        since.heard(4, Some("working"), EARLIER);
        since.heard(5, Some("waiting"), LATER);
        assert_eq!(order(&snapshot, &since), vec![5, 4]);
    }

    #[test]
    fn keeps_the_session_order_on_a_tie() {
        let first =
            workspace(2, vec![tab(3, vec![agent(20, "idle")]), tab(4, vec![agent(19, "idle"), agent(18, "idle")])]);
        let second = workspace(5, vec![tab(6, vec![agent(17, "idle")])]);
        let other = project(7, vec![workspace(8, vec![tab(9, vec![agent(16, "idle")])])]);
        let snapshot = session(vec![project(1, vec![first, second]), other]);
        assert_eq!(order(&snapshot, &Since::default()), vec![20, 19, 18, 17, 16]);
    }
}
