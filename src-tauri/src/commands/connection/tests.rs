use std::sync::Arc;

use cornercase_core::machines::MachineId;

use super::*;
use crate::commands::error::ErrorCode;
use crate::test_util::{BOOT, Recorder, Studio, changed, connected, launcher, released, serving, settle, snapshot};
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

mod without_a_machine {
    use super::*;

    #[test]
    fn there_is_no_connection_nor_session() {
        let studio = Studio::new();
        assert_eq!(current(&studio.state), ConnectionState::none());
        assert_eq!(latest_session(&studio.state), None);
    }
}
