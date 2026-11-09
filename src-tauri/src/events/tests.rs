use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use cornercase_core::cli::surface::{Colours, Size};
use cornercase_core::machines::Machine;
use cornercase_core::machines::pick::PickConfig;
use cornercase_core::machines::route::Route;
use cornercase_core::transport::Transport;
use cornercase_core::transport::fake::FakeTransport;
use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::secrets::{mock_store, on_mock};
use crate::state::AppState;
use crate::test_util::{
    APP, BOOT, CONTRACT, EVENTS, Ipc, Recorder, STATUS, Studio, TOKEN, changed, connected, contracted, heard, route,
    serving, settle, snapshot,
};

const RESTARTED: &str = "5152-1770000100";
const NO_FOLDER: Result<PathBuf, &str> = Err("no home folder");

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

fn watched(id: MachineId, fake: FakeTransport, machines: SharedMachines) -> Recorder {
    let (store, driver) = store_of(id, fake);
    let recorder = forwarding(id, &store, store.changes(), machines);
    tokio::spawn(driver);
    recorder
}

fn closed_terminal() -> Emitted {
    Emitted::Terminal(TerminalChanged {
        machine: 7,
        tab: 3,
        pane: None,
        size: Size::default(),
        cursor: None,
        colours: Colours::default(),
        replaced: false,
        rows: BTreeMap::new(),
        closed: true,
    })
}

fn closed_terminal_json() -> Value {
    let colours = json!({ "fg": null, "bg": null, "cursor": null, "palette": {} });
    let size = json!({ "cols": 0, "rows": 0 });
    json!({ "machine": 7, "tab": 3, "pane": null, "size": size, "cursor": null, "colours": colours,
        "replaced": false, "rows": {}, "closed": true })
}

fn versions(recorder: &Recorder) -> Vec<(u64, Option<String>)> {
    recorder.sessions().into_iter().map(|changed| (changed.machine, changed.session.map(|it| it.version))).collect()
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
    async fn sends_only_the_connection_and_the_session() {
        let studio = Studio::new();
        let recorder = watched(studio.id, serving(&[snapshot(BOOT, 7, "first")]), studio.state.shared_machines());
        settle().await;
        let session =
            SessionChanged { machine: studio.id.0, session: serde_json::from_str(&snapshot(BOOT, 7, "first")).ok() };
        assert!(recorder.all().contains(&Emitted::Session(session)), "{:?}", recorder.all());
        assert!(recorder.toasts().is_empty(), "{:?}", recorder.toasts());
        assert_eq!(recorder.all().len(), recorder.connections().len() + recorder.sessions().len());
    }

    #[tokio::test(start_paused = true)]
    async fn sends_one_session_for_a_burst_nobody_read_yet() {
        let studio = Studio::new();
        let burst = [snapshot(BOOT, 7, "first"), snapshot(BOOT, 8, "second"), snapshot(BOOT, 9, "third")];
        let (store, driver) = store_of(studio.id, serving(&burst));
        let changes = store.changes();
        tokio::spawn(driver);
        settle().await;
        let recorder = forwarding(studio.id, &store, changes, studio.state.shared_machines());
        settle().await;
        assert_eq!(versions(&recorder), [(studio.id.0, Some("third".to_owned()))]);
    }

    #[tokio::test(start_paused = true)]
    async fn sends_no_session_after_a_restart() {
        let studio = Studio::new();
        let recorder = watched(studio.id, restarting(), studio.state.shared_machines());
        settle().await;
        let seen = versions(&recorder);
        assert_eq!(seen.first(), Some(&(studio.id.0, Some("first".to_owned()))), "{seen:?}");
        assert_eq!(seen.last(), Some(&(studio.id.0, None)), "{seen:?}");
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

mod an_event {
    use super::*;

    #[rstest]
    #[case::connection_changed(
        Emitted::Connection(changed(MachineId(7), ConnectionView::Connecting)),
        json!({ "machine": 7, "connection": { "state": "connecting" } })
    )]
    #[case::session_changed_after_a_restart(
        Emitted::Session(SessionChanged { machine: 7, session: None }),
        json!({ "machine": 7, "session": null })
    )]
    #[case::terminal_changed(closed_terminal(), closed_terminal_json())]
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
            Emitted::Session(changed) => serde_json::to_value(changed),
            Emitted::Terminal(changed) => serde_json::to_value(changed),
            Emitted::Toast(toast) => serde_json::to_value(toast),
        };
        assert_eq!(json.expect("an event serializes"), expected);
    }

    #[test]
    fn is_named_in_kebab_case() {
        assert_eq!(
            [ConnectionChanged::NAME, SessionChanged::NAME, TerminalChanged::NAME, Toast::NAME],
            ["connection-changed", "session-changed", "terminal-changed", "toast"]
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
    #[case::session_changed(
        Emitted::Session(SessionChanged { machine: 7, session: None }),
        "session-changed",
        json!({ "machine": 7, "session": null })
    )]
    #[case::terminal_changed(closed_terminal(), "terminal-changed", closed_terminal_json())]
    #[case::toast(Emitted::Toast(Toast::error("it failed")), "toast", json!({ "level": "error", "message": "it failed" }))]
    fn emits_each_event_under_its_name(#[case] event: Emitted, #[case] name: &str, #[case] expected: Value) {
        let ipc = Ipc::new(Studio::new().reloaded());
        let heard = heard(&ipc.app, name);
        ipc.app.handle().emit(event);
        assert_eq!(heard.try_recv(), Ok(expected));
    }
}
