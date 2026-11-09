use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use cornercase_bridge::test_util::TempDir;
use cornercase_core::machines::Machine;
use cornercase_core::machines::pick::PickConfig;
use cornercase_core::machines::route::Route;
use cornercase_core::secrets::Secrets;
use cornercase_core::transport::Transport;
use cornercase_core::transport::fake::FakeTransport;
use rstest::rstest;
use serde_json::{Value, json};
use tokio::time;

use super::*;
use crate::secrets::{mock_store, on_mock};
use crate::state::AppState;

const TOKEN: &str = "c3R1ZGlvLXNlY3JldA";
const BRIDGE: &str = "ws://studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA";
const ADDRESS: &str = "ws://studio.lan:9000/";
const BOOT: &str = "5151-1770000000";
const RESTARTED: &str = "5152-1770000100";
const CONTRACT: &[&str] = &["contract", "--json"];
const EVENTS: &[&str] = &["events", "--json", "--notices"];
const STATUS: &[&str] = &["status", "--follow", "--json"];
const APP: &[&str] = &["status", "--app", "--follow", "--json"];
const NO_FOLDER: Result<PathBuf, &str> = Err("no home folder");
const REAL_CONTRACT: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../crates/core/tests/fixtures/contract/0.13.19.json"));

#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<Emitted>>>);

impl Emit for Recorder {
    fn emit(&self, event: Emitted) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).push(event);
    }
}

impl Recorder {
    fn all(&self) -> Vec<Emitted> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn connections(&self) -> Vec<ConnectionChanged> {
        self.all()
            .into_iter()
            .filter_map(|event| if let Emitted::Connection(it) = event { Some(it) } else { None })
            .collect()
    }

    fn sessions(&self) -> Vec<SessionChanged> {
        self.all()
            .into_iter()
            .filter_map(|event| if let Emitted::Session(it) = event { Some(it) } else { None })
            .collect()
    }

    fn toasts(&self) -> Vec<Toast> {
        self.all()
            .into_iter()
            .filter_map(|event| if let Emitted::Toast(it) = event { Some(it) } else { None })
            .collect()
    }
}

struct Studio {
    dir: TempDir,
    secrets: Arc<dyn Secrets>,
    state: AppState,
    id: MachineId,
}

impl Studio {
    fn new() -> Self {
        let dir = TempDir::new();
        let secrets: Arc<dyn Secrets> = Arc::new(on_mock(mock_store()));
        let state = AppState::load(Ok::<_, &str>(dir.path().to_path_buf()), Arc::clone(&secrets));
        let added = state.machines(|machines| machines.add("studio".to_owned(), route()).map_err(CommandError::from));
        Self { id: added.expect("the studio is added"), dir, secrets, state }
    }

    fn reloaded(&self) -> AppState {
        AppState::load(Ok::<_, &str>(self.dir.path().to_path_buf()), Arc::clone(&self.secrets))
    }
}

fn route() -> Route {
    Route::parse(BRIDGE).expect("a bridge url")
}

fn contract(boot: &str) -> String {
    let mut contract: Value = serde_json::from_str(REAL_CONTRACT).expect("the real contract parses");
    contract["boot"] = boot.into();
    contract.to_string()
}

fn snapshot(boot: &str, revision: u64, version: &str) -> String {
    json!({ "boot": boot, "revision": revision, "version": version }).to_string()
}

fn contracted(boot: &str) -> FakeTransport {
    FakeTransport::default().answer(CONTRACT, &[&contract(boot)], &[], 0)
}

fn serving(status: &[String]) -> FakeTransport {
    let status: Vec<&str> = status.iter().map(String::as_str).collect();
    contracted(BOOT).open(EVENTS, &[]).open(STATUS, &status).open(APP, &[&snapshot(BOOT, 1, "app")])
}

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

async fn settle() {
    time::sleep(Duration::from_secs(60)).await;
}

fn changed(id: MachineId, connection: ConnectionView) -> ConnectionChanged {
    ConnectionChanged { machine: id.0, connection }
}

fn connected() -> ConnectionView {
    ConnectionView::Connected { route: ADDRESS.to_owned() }
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
            Emitted::Toast(toast) => serde_json::to_value(toast),
        };
        assert_eq!(json.expect("an event serializes"), expected);
    }

    #[test]
    fn is_named_in_kebab_case() {
        assert_eq!(
            [ConnectionChanged::NAME, SessionChanged::NAME, Toast::NAME],
            ["connection-changed", "session-changed", "toast"]
        );
    }
}
