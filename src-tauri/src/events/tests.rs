use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use cornercase_core::cli::changes::Mode;
use cornercase_core::cli::surface::{Colours, Modes, Size};
use cornercase_core::machines::Machine;
use cornercase_core::machines::pick::PickConfig;
use cornercase_core::machines::route::Route;
use cornercase_core::protocol::{APP_ROUTES, USAGE_EXIT};
use cornercase_core::terminal::question::Choice;
use cornercase_core::transport::Transport;
use cornercase_core::transport::fake::FakeTransport;
use rstest::rstest;
use serde_json::{Value, json};
use tokio::time;

use super::*;
use crate::secrets::{mock_store, on_mock};
use crate::state::AppState;
use crate::test_util::{
    APP, BOOT, CONTRACT, EVENTS, Ipc, PAIRING, PAIRING_CODE, Recorder, STATUS, Studio, TOKEN, changed, connected,
    contracted, heard, paired_routes, route, serving, settle, snapshot,
};

const RESTARTED: &str = "5152-1770000100";
const NO_FOLDER: Result<PathBuf, &str> = Err("no home folder");
const HEARD: &str = "2026-10-16T09:30:00.123Z";
const LAN_HOST: &str = "studio.lan";
const TAILSCALE_HOST: &str = "100.101.102.103";
const TAILSCALE: &str = "ws://100.101.102.103:9000/";

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

fn telling(fake: FakeTransport, hosts: &[&str]) -> FakeTransport {
    let routes: Vec<Value> = hosts.iter().map(|host| json!({ "host": host, "port": 9000 })).collect();
    fake.answer(APP_ROUTES, &[&json!({ "routes": routes }).to_string()], &[], 0)
}

fn tailscale() -> Route {
    paired_routes(&format!("{TAILSCALE}?code={PAIRING_CODE}")).remove(0)
}

fn saved_routes(studio: &Studio) -> Vec<Route> {
    let saved = studio.reloaded().machines(|machines| Ok(machines.machine(studio.id).cloned()));
    saved.expect("the machines load").expect("the studio is saved").routes
}

fn lan_then_tailscale(id: MachineId, machines: SharedMachines) -> (Store, Recorder) {
    let lan = telling(serving(&[snapshot(BOOT, 7, "first")]), &[LAN_HOST, TAILSCALE_HOST]);
    let lan: Arc<dyn Transport> = Arc::new(by_call(lan, CONTRACT, vec![contracted(BOOT), FakeTransport::default()]));
    let tailscale: Arc<dyn Transport> =
        Arc::new(telling(serving(&[snapshot(BOOT, 8, "first")]), &[LAN_HOST, TAILSCALE_HOST]));
    let machine = Machine { id, name: "studio".to_owned(), routes: vec![route()], last_route: None };
    let transports = move |asked: &Route| match asked.address().to_string().as_str() {
        TAILSCALE => Ok(Arc::clone(&tailscale)),
        _ => Ok(Arc::clone(&lan)),
    };
    let (store, driver) = Store::new(machine, transports, PickConfig::default());
    let recorder = forwarding(id, &store, store.changes(), machines);
    tokio::spawn(driver);
    (store, recorder)
}

fn closed_terminal() -> Emitted {
    Emitted::Terminal(TerminalChanged {
        machine: 7,
        tab: 3,
        pane: None,
        size: Size::default(),
        cursor: None,
        colours: Colours::default(),
        modes: Modes::default(),
        replaced: false,
        rows: BTreeMap::new(),
        closed: true,
        unfitted: false,
    })
}

fn closed_terminal_json() -> Value {
    let colours = json!({ "fg": null, "bg": null, "cursor": null, "palette": {} });
    let size = json!({ "cols": 0, "rows": 0 });
    let modes = json!({ "mouse": "none", "mouse_encoding": "default", "alternate_screen": false,
        "alternate_scroll": false, "bracketed_paste": false, "application_cursor": false, "focus_events": false,
        "kitty_keyboard": 0 });
    json!({ "machine": 7, "tab": 3, "pane": null, "size": size, "cursor": null, "colours": colours,
        "modes": modes, "replaced": false, "rows": {}, "closed": true, "unfitted": false })
}

fn asked() -> Emitted {
    let options = vec![Choice { number: 1, label: "Yes".to_owned() }, Choice { number: 2, label: "No".to_owned() }];
    Emitted::Question(QuestionChanged {
        machine: 7,
        tab: 3,
        pane: 4,
        colours: Colours::default(),
        rows: Vec::new(),
        options,
        card: None,
        closed: false,
    })
}

fn asked_json() -> Value {
    let colours = json!({ "fg": null, "bg": null, "cursor": null, "palette": {} });
    let options = json!([{ "number": 1, "label": "Yes" }, { "number": 2, "label": "No" }]);
    json!({ "machine": 7, "tab": 3, "pane": 4, "colours": colours, "rows": [], "options": options, "card": null,
        "closed": false })
}

fn closed_changes() -> Emitted {
    Emitted::Changes(ChangesChanged { machine: 7, workspace: 2, mode: Mode::Commits, summary: None, closed: true })
}

fn closed_changes_json() -> Value {
    json!({ "machine": 7, "workspace": 2, "mode": "commits", "summary": null, "closed": true })
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
        let state = AppState::load(NO_FOLDER, &on_mock(mock_store()), Arc::new(on_mock(mock_store())));
        let id = MachineId(5);
        let recorder = watched(id, serving(&[snapshot(BOOT, 7, "first")]), state.shared_machines());
        settle().await;
        assert_eq!(recorder.connections(), [changed(id, ConnectionView::Connecting), changed(id, connected())]);
        assert_eq!(recorder.toasts(), [Toast::error("the app has no folder for its data: no home folder")]);
    }
}

mod learning_the_routes {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn saves_the_address_the_machine_lacked_with_the_token_of_the_route_that_won() {
        let studio = Studio::new();
        let fake = telling(serving(&[snapshot(BOOT, 7, "first")]), &[LAN_HOST, TAILSCALE_HOST]);
        let recorder = watched(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(saved_routes(&studio), [route(), tailscale()]);
        assert_eq!(
            recorder.connections(),
            [changed(studio.id, ConnectionView::Connecting), changed(studio.id, connected())]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn lets_the_store_race_the_learned_address_once_the_network_changes() {
        let studio = Studio::new();
        let (store, recorder) = lan_then_tailscale(studio.id, studio.state.shared_machines());
        settle().await;
        store.network_changed();
        settle().await;
        let over_tailscale = ConnectionView::Connected { route: TAILSCALE.to_owned() };
        assert_eq!(recorder.connections().last(), Some(&changed(studio.id, over_tailscale)));
        assert_eq!(saved_routes(&studio), [route(), tailscale()]);
    }

    #[tokio::test(start_paused = true)]
    async fn adds_an_address_once_however_often_the_mac_tells_it() {
        let studio = Studio::new();
        let (store, _driver) = store_of(studio.id, serving(&[]));
        let transport: Arc<dyn Transport> =
            Arc::new(telling(FakeTransport::default(), &[TAILSCALE_HOST, TAILSCALE_HOST]));
        for _ in 0..2 {
            learn(studio.id, Arc::clone(&transport), route(), store.clone(), studio.state.shared_machines()).await;
        }
        assert_eq!(saved_routes(&studio), [route(), tailscale()]);
    }

    #[tokio::test(start_paused = true)]
    async fn saves_the_addresses_a_mac_paired_with_cornercase_tells_with_its_secret() {
        let studio = Studio::new();
        let paired = paired_routes(PAIRING);
        let merged = studio.state.machines(|machines| machines.merge(studio.id, &paired).map_err(CommandError::from));
        merged.expect("the paired routes are merged");
        let (store, _driver) = store_of(studio.id, serving(&[]));
        let told =
            json!({ "routes": [{ "host": LAN_HOST, "port": 47682 }, { "host": TAILSCALE_HOST, "port": 47682 }] });
        let transport: Arc<dyn Transport> =
            Arc::new(FakeTransport::default().answer(APP_ROUTES, &[&told.to_string()], &[], 0));
        learn(studio.id, transport, paired[0].clone(), store, studio.state.shared_machines()).await;
        let tailscale = paired_routes(&format!("ws://{TAILSCALE_HOST}:47682/?code={PAIRING_CODE}"));
        assert_eq!(saved_routes(&studio), [vec![route()], paired, tailscale].concat());
    }

    #[rstest]
    #[case::refused(serving(&[snapshot(BOOT, 7, "first")]).answer(APP_ROUTES, &[], &["unknown operation app-routes"], USAGE_EXIT))]
    #[case::not_json(serving(&[snapshot(BOOT, 7, "first")]).answer(APP_ROUTES, &["usage"], &[], 0))]
    #[case::unanswered(serving(&[snapshot(BOOT, 7, "first")]))]
    #[tokio::test(start_paused = true)]
    async fn stays_quiet_with_a_bridge_that_does_not_know_the_request(#[case] fake: FakeTransport) {
        let studio = Studio::new();
        let recorder = watched(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(recorder.all().len(), recorder.connections().len() + recorder.agents().len());
        assert_eq!(
            recorder.connections(),
            [changed(studio.id, ConnectionView::Connecting), changed(studio.id, connected())]
        );
        assert_eq!(saved_routes(&studio), [route()]);
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_forwarding_while_the_bridge_has_not_told_them() {
        let studio = Studio::new();
        let fake = one_by_one(&[with_claude(7, "working"), with_claude(8, "waiting")]).open(APP_ROUTES, &[]);
        let recorder = watched(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(rows(&recorder), [Some(claude("working", None)), Some(claude("waiting", None))]);
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

    #[tokio::test(start_paused = true)]
    async fn are_not_sent_again_for_a_status_event_ahead_of_the_snapshot() {
        let studio = Studio::new();
        let events = [status_event("working"), status_event("waiting")];
        let fake = later(serving(&[with_claude(7, "working")]), EVENTS, &events);
        let recorder = watched(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(rows(&recorder), [Some(claude("working", None)), Some(claude("working", Some(HEARD)))]);
    }

    #[tokio::test(start_paused = true)]
    async fn are_sent_again_without_their_times_once_some_events_were_dropped() {
        let studio = Studio::new();
        let dropped = json!({ "time": HEARD, "event": "dropped", "count": 3 }).to_string();
        let fake = later(serving(&[with_claude(7, "waiting")]), EVENTS, &[status_event("waiting"), dropped]);
        let recorder = watched(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(rows(&recorder).last(), Some(&Some(claude("waiting", None))));
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

mod a_pane_status {
    use super::*;

    async fn panes_heard_after(event: String) -> Vec<Option<u64>> {
        let studio = Studio::new();
        let fake = later(serving(&[with_claude(7, "waiting")]), EVENTS, &[event]);
        let recorder = watched(studio.id, fake, studio.state.shared_machines());
        settle().await;
        recorder.pane_statuses().into_iter().map(|changed| changed.pane).collect()
    }

    #[tokio::test(start_paused = true)]
    async fn is_sent_for_the_pane_of_a_status_event_along_with_the_agents() {
        let studio = Studio::new();
        let fake = later(serving(&[with_claude(7, "waiting")]), EVENTS, &[status_event("waiting")]);
        let recorder = watched(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(recorder.pane_statuses(), [PaneStatusChanged { machine: studio.id.0, pane: Some(4) }]);
        assert_eq!(rows(&recorder), [Some(claude("waiting", None)), Some(claude("waiting", Some(HEARD)))]);
    }

    #[tokio::test(start_paused = true)]
    async fn is_sent_for_no_pane_once_some_events_were_dropped() {
        let dropped = json!({ "time": HEARD, "event": "dropped", "count": 3 });
        assert_eq!(panes_heard_after(dropped.to_string()).await, [None]);
    }

    #[tokio::test(start_paused = true)]
    async fn is_sent_for_no_pane_when_the_events_are_opened_again() {
        let studio = Studio::new();
        let fake = serving(&[with_claude(7, "waiting")]);
        let (store, recorder) = watching(studio.id, fake, studio.state.shared_machines());
        settle().await;
        assert_eq!(recorder.pane_statuses(), []);
        store.resume_now();
        settle().await;
        assert_eq!(recorder.pane_statuses(), [PaneStatusChanged { machine: studio.id.0, pane: None }]);
    }

    #[rstest]
    #[case::opened(json!({ "time": HEARD, "event": "opened", "kind": "tab", "project": 1, "workspace": 2, "tab": 3 }))]
    #[case::notice(json!({
        "time": HEARD, "event": "notice", "token": "cc-ab12cd-3", "agent": "claude", "status": "waiting",
        "place": "shop › default", "message": "claude needs you in shop › default", "panes": [4],
        "project": 1, "workspace": 2,
    }))]
    #[case::status_without_a_pane(json!({ "time": HEARD, "event": "status", "agent": "claude", "to": "waiting" }))]
    #[tokio::test(start_paused = true)]
    async fn is_not_sent_for_an_event_about_no_agent_pane(#[case] event: Value) {
        assert_eq!(panes_heard_after(event.to_string()).await, []);
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
    #[case::pane_status_changed_after_a_drop(
        Emitted::PaneStatus(PaneStatusChanged { machine: 7, pane: None }),
        json!({ "machine": 7, "pane": null })
    )]
    #[case::terminal_changed(closed_terminal(), closed_terminal_json())]
    #[case::question_changed(asked(), asked_json())]
    #[case::changes_changed_once_the_workspace_closed(closed_changes(), closed_changes_json())]
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
            Emitted::PaneStatus(changed) => serde_json::to_value(changed),
            Emitted::Terminal(changed) => serde_json::to_value(changed),
            Emitted::Question(changed) => serde_json::to_value(changed),
            Emitted::Changes(changed) => serde_json::to_value(changed),
            Emitted::Toast(toast) => serde_json::to_value(toast),
        };
        assert_eq!(json.expect("an event serializes"), expected);
    }

    #[test]
    fn is_named_in_kebab_case() {
        let names = [
            ConnectionChanged::NAME,
            AgentsChanged::NAME,
            PaneStatusChanged::NAME,
            TerminalChanged::NAME,
            QuestionChanged::NAME,
            ChangesChanged::NAME,
            Toast::NAME,
        ];
        let kebab = [
            "connection-changed",
            "agents-changed",
            "pane-status-changed",
            "terminal-changed",
            "question-changed",
            "changes-changed",
            "toast",
        ];
        assert_eq!(names, kebab);
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
    #[case::pane_status_changed(
        Emitted::PaneStatus(PaneStatusChanged { machine: 7, pane: Some(4) }),
        "pane-status-changed",
        json!({ "machine": 7, "pane": 4 })
    )]
    #[case::terminal_changed(closed_terminal(), "terminal-changed", closed_terminal_json())]
    #[case::question_changed(asked(), "question-changed", asked_json())]
    #[case::changes_changed(closed_changes(), "changes-changed", closed_changes_json())]
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
    #[case::dropped(event(What::Dropped { count: 3 }), true)]
    #[case::exited(event(What::Exited { program: "sh".into() }), false)]
    #[case::app(Change::App, false)]
    #[case::reopened(Change::Reopened, false)]
    fn touches_the_agents_only_when_a_row_may_change(#[case] change: Change, #[case] touches: bool) {
        assert_eq!(touches_agents(&change), touches);
    }
}
