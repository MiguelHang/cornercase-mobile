use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use cornercase_bridge::test_util::TempDir;
use cornercase_core::machines::MachineId;
use cornercase_core::machines::route::Route;
use cornercase_core::secrets::Secrets;
use cornercase_core::transport::Transport;
use cornercase_core::transport::fake::FakeTransport;
use serde_json::{Value, json};
use tauri::async_runtime::JoinHandle;
use tokio::time;

use crate::commands::connection::Launcher;
use crate::commands::error::CommandError;
use crate::events::{ConnectionChanged, Emit, Emitted, SessionChanged, Toast};
use crate::secrets::{mock_store, on_mock};
use crate::state::AppState;
use crate::view::ConnectionView;

pub(crate) const TOKEN: &str = "c3R1ZGlvLXNlY3JldA";
pub(crate) const BRIDGE: &str = "ws://studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA";
pub(crate) const ADDRESS: &str = "ws://studio.lan:9000/";
pub(crate) const BOOT: &str = "5151-1770000000";
pub(crate) const CONTRACT: &[&str] = &["contract", "--json"];
pub(crate) const EVENTS: &[&str] = &["events", "--json", "--notices"];
pub(crate) const STATUS: &[&str] = &["status", "--follow", "--json"];
pub(crate) const APP: &[&str] = &["status", "--app", "--follow", "--json"];
const REAL_CONTRACT: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../crates/core/tests/fixtures/contract/0.13.19.json"));

#[derive(Clone, Default)]
pub(crate) struct Recorder(Arc<Mutex<Vec<Emitted>>>);

impl Emit for Recorder {
    fn emit(&self, event: Emitted) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).push(event);
    }
}

impl Recorder {
    pub(crate) fn all(&self) -> Vec<Emitted> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub(crate) fn connections(&self) -> Vec<ConnectionChanged> {
        self.all()
            .into_iter()
            .filter_map(|event| if let Emitted::Connection(it) = event { Some(it) } else { None })
            .collect()
    }

    pub(crate) fn sessions(&self) -> Vec<SessionChanged> {
        self.all()
            .into_iter()
            .filter_map(|event| if let Emitted::Session(it) = event { Some(it) } else { None })
            .collect()
    }

    pub(crate) fn toasts(&self) -> Vec<Toast> {
        self.all()
            .into_iter()
            .filter_map(|event| if let Emitted::Toast(it) = event { Some(it) } else { None })
            .collect()
    }
}

pub(crate) struct Studio {
    pub(crate) dir: TempDir,
    pub(crate) secrets: Arc<dyn Secrets>,
    pub(crate) state: AppState,
    pub(crate) id: MachineId,
}

impl Studio {
    pub(crate) fn new() -> Self {
        let dir = TempDir::new();
        let secrets: Arc<dyn Secrets> = Arc::new(on_mock(mock_store()));
        let state = AppState::load(Ok::<_, &str>(dir.path().to_path_buf()), Arc::clone(&secrets));
        let added = state.machines(|machines| machines.add("studio".to_owned(), route()).map_err(CommandError::from));
        Self { id: added.expect("the studio is added"), dir, secrets, state }
    }

    pub(crate) fn reloaded(&self) -> AppState {
        AppState::load(Ok::<_, &str>(self.dir.path().to_path_buf()), Arc::clone(&self.secrets))
    }
}

pub(crate) fn route() -> Route {
    Route::parse(BRIDGE).expect("a bridge url")
}

pub(crate) fn snapshot(boot: &str, revision: u64, version: &str) -> String {
    json!({ "boot": boot, "revision": revision, "version": version }).to_string()
}

pub(crate) fn contracted(boot: &str) -> FakeTransport {
    let mut contract: Value = serde_json::from_str(REAL_CONTRACT).expect("the real contract parses");
    contract["boot"] = boot.into();
    FakeTransport::default().answer(CONTRACT, &[&contract.to_string()], &[], 0)
}

pub(crate) fn serving(status: &[String]) -> FakeTransport {
    let status: Vec<&str> = status.iter().map(String::as_str).collect();
    contracted(BOOT).open(EVENTS, &[]).open(STATUS, &status).open(APP, &[&snapshot(BOOT, 1, "app")])
}

pub(crate) fn changed(id: MachineId, connection: ConnectionView) -> ConnectionChanged {
    ConnectionChanged { machine: id.0, connection }
}

pub(crate) fn connected() -> ConnectionView {
    ConnectionView::Connected { route: ADDRESS.to_owned() }
}

pub(crate) async fn settle() {
    time::sleep(Duration::from_secs(60)).await;
}

pub(crate) async fn released<T>(held: &Arc<T>) {
    time::timeout(Duration::from_secs(60), async {
        while Arc::strong_count(held) > 1 {
            time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the tasks let go within a minute");
}

pub(crate) fn launcher(fake: FakeTransport) -> (Launcher, Arc<FakeTransport>) {
    let transport = Arc::new(fake);
    let held = Arc::clone(&transport);
    let launcher = Launcher::new(
        move |_: &Route| Ok(Arc::clone(&held) as Arc<dyn Transport>),
        |task| JoinHandle::Tokio(tokio::spawn(task)),
    );
    (launcher, transport)
}
