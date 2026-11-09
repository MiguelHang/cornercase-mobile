use super::*;
use crate::cli::status::{Project, Tab, Workspace};

const EARLIER: &str = "2026-11-09T12:00:00.000Z";
const LATER: &str = "2026-11-09T12:05:00.000Z";

fn pane(id: u64, agent: Option<&str>, status: Option<&str>) -> Pane {
    Pane { id, agent: agent.map(str::to_owned), status: status.map(str::to_owned), ..Pane::default() }
}

fn agent(id: u64, status: &str) -> Pane {
    pane(id, Some("claude"), Some(status))
}

fn snapshot(panes: Vec<Pane>) -> Snapshot {
    let tab = Tab { id: 3, panes, ..Tab::default() };
    let workspace = Workspace { id: 2, tabs: vec![tab], ..Workspace::default() };
    let project = Project { id: 1, workspaces: vec![workspace], ..Project::default() };
    Snapshot { boot: "1-2".to_owned(), projects: vec![project], ..Snapshot::default() }
}

fn seen(panes: Vec<Pane>) -> Since {
    let mut since = Since::default();
    since.reconcile(&snapshot(panes));
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
        assert_eq!(since.of(4, Some("waiting")), Some(LATER));
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
