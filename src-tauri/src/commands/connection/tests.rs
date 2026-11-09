use std::sync::Arc;

use cornercase_core::machines::MachineId;
use cornercase_core::transport::fake::FakeTransport;

use super::*;
use crate::commands::error::ErrorCode;
use crate::test_util::{
    BOOT, Ipc, Recorder, Studio, changed, connected, launcher, released, serving, settle, snapshot,
};
use crate::view::ConnectionView;

fn first_snapshot() -> Vec<String> {
    vec![snapshot(BOOT, 7, "first")]
}

mod connecting {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn says_the_machine_is_unknown() {
        let studio = Studio::new();
        let (launcher, _) = launcher(serving(&first_snapshot()));
        let error = start(&studio.state, Recorder::default(), &launcher, MachineId(99)).expect_err("no such machine");
        assert_eq!(error.code, ErrorCode::UnknownMachine);
        assert!(studio.state.active().is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn forwards_the_changes_of_its_store() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        let (launcher, _) = launcher(serving(&first_snapshot()));
        start(&studio.state, recorder.clone(), &launcher, studio.id).expect("connected");
        settle().await;
        assert_eq!(
            recorder.connections(),
            [changed(studio.id, ConnectionView::Connecting), changed(studio.id, connected())]
        );
        assert_eq!(current(&studio.state), ConnectionState { machine: Some(studio.id.0), connection: connected() });
        assert_eq!(latest_session(&studio.state).map(|session| session.version), Some("first".to_owned()));
    }

    #[tokio::test(start_paused = true)]
    async fn twice_makes_a_new_store_and_ends_the_old_one() {
        let studio = Studio::new();
        let (first, first_transport) = launcher(serving(&first_snapshot()));
        start(&studio.state, Recorder::default(), &first, studio.id).expect("connected");
        drop(first);
        settle().await;
        let (second, second_transport) = launcher(serving(&first_snapshot()));
        start(&studio.state, Recorder::default(), &second, studio.id).expect("connected again");
        released(&first_transport).await;
        assert!(Arc::strong_count(&second_transport) > 2);
        assert_eq!(studio.state.active().as_ref().map(Active::machine), Some(studio.id));
    }
}

mod with_the_network {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use cornercase_core::network::{Kind, Network};

    use super::*;

    const NONE: Network = Network { kind: Kind::None, expensive: false };
    const WIFI: Network = Network { kind: Kind::Wifi, expensive: false };

    fn counting(fake: FakeTransport) -> (Launcher, Arc<AtomicUsize>) {
        let transport: Arc<dyn Transport> = Arc::new(fake);
        let tries = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&tries);
        let launcher = Launcher::new(
            move |_: &Route| {
                counted.fetch_add(1, Ordering::SeqCst);
                Ok(Arc::clone(&transport))
            },
            |task| JoinHandle::Tokio(tokio::spawn(task)),
        );
        (launcher, tries)
    }

    #[tokio::test(start_paused = true)]
    async fn down_starts_the_new_store_offline_without_trying() {
        let studio = Studio::new();
        studio.state.replace_network(NONE);
        let recorder = Recorder::default();
        let (launcher, tries) = counting(serving(&first_snapshot()));
        start(&studio.state, recorder.clone(), &launcher, studio.id).expect("started");
        settle().await;
        assert_eq!(recorder.connections(), [changed(studio.id, ConnectionView::Offline)]);
        assert_eq!(tries.load(Ordering::SeqCst), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn down_connects_once_it_comes_back() {
        let studio = Studio::new();
        studio.state.replace_network(NONE);
        let recorder = Recorder::default();
        let (launcher, _) = counting(serving(&first_snapshot()));
        start(&studio.state, recorder.clone(), &launcher, studio.id).expect("started");
        settle().await;
        studio.state.active().as_ref().expect("an active store").store().online();
        settle().await;
        assert_eq!(recorder.connections().last(), Some(&changed(studio.id, connected())));
    }

    #[tokio::test(start_paused = true)]
    async fn up_connects_as_before() {
        let studio = Studio::new();
        studio.state.replace_network(WIFI);
        let recorder = Recorder::default();
        let (launcher, tries) = counting(serving(&first_snapshot()));
        start(&studio.state, recorder.clone(), &launcher, studio.id).expect("started");
        settle().await;
        assert_eq!(
            recorder.connections(),
            [changed(studio.id, ConnectionView::Connecting), changed(studio.id, connected())]
        );
        assert!(tries.load(Ordering::SeqCst) > 0);
    }
}

mod without_a_machine {
    use super::*;

    #[test]
    fn there_is_no_connection_nor_session() {
        let studio = Studio::new();
        assert_eq!(current(&studio.state), ConnectionState::none());
        assert_eq!(latest_session(&studio.state), None);
    }
}

mod over_ipc {
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn says_the_machine_to_connect_to_is_unknown() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        let refused = ipc.invoke("connect", json!({ "id": 99 })).expect_err("no such machine");
        assert_eq!(refused["code"], "unknown_machine");
    }

    #[test]
    fn answers_there_is_no_connection_nor_session() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        let state = ipc.invoke("connection_state", json!({})).expect("a connection state");
        assert_eq!(state, json!({ "machine": null, "connection": { "state": "none" } }));
        assert_eq!(ipc.invoke("session", json!({})), Ok(Value::Null));
    }

    #[tokio::test(start_paused = true)]
    async fn answers_the_active_connection_and_its_session() {
        let studio = Studio::new();
        let state = studio.reloaded();
        let (launcher, _) = launcher(serving(&first_snapshot()));
        start(&state, Recorder::default(), &launcher, studio.id).expect("connecting");
        settle().await;
        let ipc = Ipc::new(state);
        let state = ipc.invoke("connection_state", json!({})).expect("a connection state");
        assert_eq!(state["connection"]["state"], "connected");
        assert_eq!(state["machine"], studio.id.0);
        let session = ipc.invoke("session", json!({})).expect("a session");
        assert_eq!(session["version"], "first");
    }
}
