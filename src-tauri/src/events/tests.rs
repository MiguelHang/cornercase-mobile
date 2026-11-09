use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use cornercase_core::machines::Machine;
use cornercase_core::machines::pick::PickConfig;
use cornercase_core::machines::route::Route;
use cornercase_core::transport::Transport;
use cornercase_core::transport::fake::FakeTransport;
use rstest::rstest;
use serde_json::{Value, json};
use tokio::time;

use super::*;
use crate::secrets::{mock_store, on_mock};
use crate::state::AppState;
use crate::test_util::{
    APP, BOOT, CONTRACT, EVENTS, Ipc, Recorder, STATUS, Studio, TOKEN, changed, connected, contracted, heard, route,
    serving, settle, snapshot,
};

const RESTARTED: &str = "5152-1770000100";
const NO_FOLDER: Result<PathBuf, &str> = Err("no home folder");
const HEARD: &str = "2026-10-16T09:30:00.123Z";

type Row = (Option<String>, Option<String>);

fn by_call(fake: FakeTransport, argv: &'static [&'static str], answers: Vec<FakeTransport>) -> FakeTransport {
    let answers: Vec<Arc<FakeTransport>> = answers.into_iter().map(Arc::new).collect();
    let calls = AtomicUsize::new(0);
    fake.script(argv, move |stdin, out| {
        let call = calls.fetch_add(1, Ordering::SeqCst);
        let answer = Arc::clone(answers.get(call).or(answers.last()).expect("an answer for every call"));
        let argv = argv.iter().map(|&word| word.to_owned()).collect();
        async move { answer.run(argv, stdin, out).await }
    })
}

fn restarting() -> FakeTransport {
    let contracts = vec![contracted(BOOT), contracted(RESTARTED)];
    let first = FakeTransport::default().answer(STATUS, &[&snapshot(BOOT, 7, "first")], &[], 0);
    let watches = vec![first, FakeTransport::default().open(STATUS, &[])];
    let fake = FakeTransport::default().open(EVENTS, &[]).open(APP, &[]);
    by_call(by_call(fake, CONTRACT, contracts), STATUS, watches)
}

fn later(fake: FakeTransport, argv: &'static [&'static str], lines: &[String]) -> FakeTransport {
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let late = Arc::new(FakeTransport::default().open(argv, &lines));
    fake.script(argv, move |stdin, out| {
        let late = Arc::clone(&late);
        let argv = argv.iter().map(|&word| word.to_owned()).collect();
        async move {
            time::sleep(Duration::from_secs(5)).await;
            late.run(argv, stdin, out).await
        }
    })
}

fn one_by_one(snapshots: &[String]) -> FakeTransport {
    let mut watches: Vec<FakeTransport> =
        snapshots.iter().map(|snapshot| FakeTransport::default().answer(STATUS, &[snapshot], &[], 0)).collect();
    watches.push(FakeTransport::default().open(STATUS, &[]));
    by_call(serving(&[]), STATUS, watches)
}

fn with_claude(revision: u64, status: &str) -> String {
    let pane = json!({ "id": 4, "agent": "claude", "status": status });
    let tab = json!({ "id": 3, "name": "claude", "panes": [pane] });
    let workspace = json!({ "id": 2, "name": "default", "tabs": [tab] });
    let project = json!({ "id": 1, "name": "shop", "workspaces": [workspace] });
    json!({ "boot": BOOT, "revision": revision, "version": "first", "projects": [project] }).to_string()
}

fn status_event(to: &str) -> String {
    json!({
        "time": HEARD, "event": "status", "agent": "claude", "from": "working", "to": to,
        "project": 1, "workspace": 2, "tab": 3, "pane": 4,
    })
    .to_string()
}

fn store_of(id: MachineId, fake: FakeTransport) -> (Store, impl Future<Output = ()> + Send + 'static) {
    let transport: Arc<dyn Transport> = Arc::new(fake);
    let machine = Machine { id, name: "studio".to_owned(), routes: vec![route()], last_route: None };
    Store::new(machine, move |_: &Route| Ok(Arc::clone(&transport)), PickConfig::default())
}

fn forwarding(id: MachineId, store: &Store, changes: Changes, machines: SharedMachines) -> Recorder {
    let recorder = Recorder::default();
    tokio::spawn(forward(id, store.clone(), changes, machines, recorder.clone()));
    recorder
}

fn watching(id: MachineId, fake: FakeTransport, machines: SharedMachines) -> (Store, Recorder) {
    let (store, driver) = store_of(id, fake);
    let recorder = forwarding(id, &store, store.changes(), machines);
    tokio::spawn(driver);
    (store, recorder)
}

fn watched(id: MachineId, fake: FakeTransport, machines: SharedMachines) -> Recorder {
    watching(id, fake, machines).1
}

fn rows(recorder: &Recorder) -> Vec<Option<Vec<Row>>> {
    let row = |row: AgentRow| (row.status, row.since);
    recorder.agents().into_iter().map(|changed| changed.agents.map(|it| it.into_iter().map(row).collect())).collect()
}

fn claude(status: &str, since: Option<&str>) -> Vec<Row> {
    vec![(Some(status.to_owned()), since.map(str::to_owned))]
}

mod the_forwarder {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn announces_the_connection_and_saves_the_route_that_worked() {
        let studio = Studio::new();
        let recorder = watched(studio.id, serving(&[snapshot(BOOT, 7, "first")]), studio.state.shared_machines());
        settle().await;
        let expected = [changed(studio.id, ConnectionView::Connecting), changed(studio.id, connected())];
        assert_eq!(recorder.connections(), expected);
        let remembered = studio.reloaded().machines(|machines| Ok(machines.machine(studio.id).cloned()));
        let last_route = remembered.expect("the machines load").and_then(|machine| machine.last_route);
        assert_eq!(last_route, Some(route().address().clone()));
    }

    #[tokio::test(start_paused = true)]
    async fn sends_only_the_connection_and_the_agents() {
        let studio = Studio::new();
        let recorder = watched(studio.id, serving(&[snapshot(BOOT, 7, "first")]), studio.state.shared_machines());
        settle().await;
        assert!(recorder.toasts().is_empty(), "{:?}", recorder.toasts());
        let agents = AgentsChanged { machine: studio.id.0, agents: Some(Vec::new()) };
        assert_eq!(recorder.agents(), [agents]);
        assert_eq!(recorder.all().len(), recorder.connections().len() + recorder.agents().len());
    }

    #[tokio::test(start_paused = true)]
    async fn sends_the_agents_once_for_a_burst_nobody_read_yet() {
        let studio = Studio::new();
        let burst = [with_claude(7, "working"), with_claude(8, "waiting"), with_claude(9, "done")];
        let (store, driver) = store_of(studio.id, serving(&burst));
        let changes = store.changes();
        tokio::spawn(driver);
        settle().await;
        let recorder = forwarding(studio.id, &store, changes, studio.state.shared_machines());
        settle().await;
        assert_eq!(rows(&recorder), [Some(claude("done", None))]);
    }

    #[tokio::test(start_paused = true)]
    async fn turns_a_warning_into_a_toast_without_the_token_or_what_was_typed() {
        let studio = Studio::new();
        let broken = format!(r#"{{"token": {TOKEN}, "typed": "rm -rf ~"#);
        let recorder =
            watched(studio.id, serving(&[broken, snapshot(BOOT, 7, "first")]), studio.state.shared_machines());
        settle().await;
        let toasts = recorder.toasts();
        assert!(matches!(&toasts[..], [Toast { level: ToastLevel::Error, .. }]), "{toasts:?}");
        assert!(toasts.iter().all(|toast| !toast.message.contains(TOKEN) && !toast.message.contains("rm -rf")));
    }

    #[tokio::test(start_paused = true)]
    async fn says_why_the_route_that_worked_was_not_saved() {
        let state = AppState::load(NO_FOLDER, Arc::new(on_mock(mock_store())));
        let id = MachineId(5);
        let recorder = watched(id, serving(&[snapshot(BOOT, 7, "first")]), state.shared_machines());
        settle().await;
        assert_eq!(recorder.connections(), [changed(id, ConnectionView::Connecting), changed(id, connected())]);
        assert_eq!(recorder.toasts(), [Toast::error("the app has no folder for its data: no home folder")]);
    }
}

mod the_agents {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn are_sent_once_while_the_snapshots_keep_the_same_rows() {
        let studio = Studio::new();
        let fake = one_by_one(&[with_claude(7, "working"), with_claude(8, "working")]);
        let (store, recorder) = watching(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(store.session().map(|session| session.revision), Some(8));
        assert_eq!(rows(&recorder), [Some(claude("working", None))]);
    }

    #[tokio::test(start_paused = true)]
    async fn are_sent_again_when_an_agent_changes_its_status() {
        let studio = Studio::new();
        let fake = one_by_one(&[with_claude(7, "working"), with_claude(8, "waiting")]);
        let recorder = watched(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(rows(&recorder), [Some(claude("working", None)), Some(claude("waiting", None))]);
    }

    #[tokio::test(start_paused = true)]
    async fn are_sent_again_when_a_status_event_says_since_when() {
        let studio = Studio::new();
        let fake = later(serving(&[with_claude(7, "waiting")]), EVENTS, &[status_event("waiting")]);
        let recorder = watched(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(rows(&recorder), [Some(claude("waiting", None)), Some(claude("waiting", Some(HEARD)))]);
    }

    #[rstest]
    #[case::opened(json!({ "time": HEARD, "event": "opened", "kind": "tab", "project": 1, "workspace": 2, "tab": 3 }))]
    #[case::notice(json!({
        "time": HEARD, "event": "notice", "token": "cc-ab12cd-3", "agent": "claude", "status": "waiting",
        "place": "shop › default", "message": "claude needs you in shop › default", "panes": [4],
        "project": 1, "workspace": 2,
    }))]
    #[tokio::test(start_paused = true)]
    async fn are_not_sent_again_for_an_event_that_touches_no_row(#[case] event: Value) {
        let studio = Studio::new();
        let fake = later(serving(&[with_claude(7, "waiting")]), EVENTS, &[event.to_string()]);
        let recorder = watched(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(rows(&recorder), [Some(claude("waiting", None))]);
    }

    #[tokio::test(start_paused = true)]
    async fn are_sent_as_none_after_a_restart() {
        let studio = Studio::new();
        let recorder = watched(studio.id, restarting(), studio.state.shared_machines());
        settle().await;
        assert_eq!(rows(&recorder), [Some(Vec::new()), None]);
    }
}

mod an_event {
    use super::*;

    #[rstest]
    #[case::connection_changed(
        Emitted::Connection(changed(MachineId(7), ConnectionView::Connecting)),
        json!({ "machine": 7, "connection": { "state": "connecting" } })
    )]
    #[case::agents_changed_after_a_restart(
        Emitted::Agents(AgentsChanged { machine: 7, agents: None }),
        json!({ "machine": 7, "agents": null })
    )]
    #[case::error_toast(
        Emitted::Toast(Toast::error("it failed")),
        json!({ "level": "error", "message": "it failed" })
    )]
    #[case::info_toast(
        Emitted::Toast(Toast { level: ToastLevel::Info, message: "saved".to_owned() }),
        json!({ "level": "info", "message": "saved" })
    )]
    fn carries_its_fields(#[case] event: Emitted, #[case] expected: Value) {
        let json = match event {
            Emitted::Connection(changed) => serde_json::to_value(changed),
            Emitted::Agents(changed) => serde_json::to_value(changed),
            Emitted::Toast(toast) => serde_json::to_value(toast),
        };
        assert_eq!(json.expect("an event serializes"), expected);
    }

    #[test]
    fn is_named_in_kebab_case() {
        assert_eq!(
            [ConnectionChanged::NAME, AgentsChanged::NAME, Toast::NAME],
            ["connection-changed", "agents-changed", "toast"]
        );
    }
}

mod the_app {
    use super::*;

    #[rstest]
    #[case::connection_changed(
        Emitted::Connection(changed(MachineId(7), ConnectionView::Connecting)),
        "connection-changed",
        json!({ "machine": 7, "connection": { "state": "connecting" } })
    )]
    #[case::agents_changed(
        Emitted::Agents(AgentsChanged { machine: 7, agents: Some(Vec::new()) }),
        "agents-changed",
        json!({ "machine": 7, "agents": [] })
    )]
    #[case::toast(Emitted::Toast(Toast::error("it failed")), "toast", json!({ "level": "error", "message": "it failed" }))]
    fn emits_each_event_under_its_name(#[case] event: Emitted, #[case] name: &str, #[case] expected: Value) {
        let ipc = Ipc::new(Studio::new().reloaded());
        let heard = heard(&ipc.app, name);
        ipc.app.handle().emit(event);
        assert_eq!(heard.try_recv(), Ok(expected));
    }
}

mod a_change {
    use cornercase_core::cli::events::Event;

    use super::*;

    fn event(what: What) -> Change {
        Change::Event(Box::new(Event {
            time: HEARD.to_owned(),
            project: None,
            workspace: None,
            tab: None,
            pane: Some(4),
            what,
        }))
    }

    #[rstest]
    #[case::session(Change::Session, true)]
    #[case::status(event(What::Status { agent: "claude".into(), from: None, to: Some("waiting".into()) }), true)]
    #[case::exited(event(What::Exited { program: "sh".into() }), false)]
    #[case::app(Change::App, false)]
    fn touches_the_agents_only_when_a_row_may_change(#[case] change: Change, #[case] touches: bool) {
        assert_eq!(touches_agents(&change), touches);
    }
}
