use cornercase_core::error::Error;
use cornercase_core::transport::fake::FakeTransport;
use rstest::rstest;
use serde_json::{Value, json};

use crate::commands::connection::start;
use crate::test_util::{BOOT, Ipc, Recorder, Studio, launcher, serving, settle, snapshot};

const READ: &[&str] = &["read", "--last-message", "--pane", "543", "--json"];
const TURN_OVER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/last-message/0.13.19/turn-over.json"
));
const NO_PANE: &str = "Error: there is no pane 543; `cornercase status` lists them";

async fn connected_with(read: impl FnOnce(FakeTransport) -> FakeTransport) -> Ipc {
    let studio = Studio::new();
    let state = studio.reloaded();
    let (launcher, _) = launcher(read(serving(&[snapshot(BOOT, 7, "first")])));
    start(&state, Recorder::default(), &launcher, studio.id).expect("connecting");
    settle().await;
    Ipc::new(state)
}

mod reading_the_last_message {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn answers_what_cornercase_printed_for_the_pane() {
        let lines: Vec<&str> = TURN_OVER.lines().collect();
        let ipc = connected_with(|fake| fake.answer(READ, &lines, &[], 0)).await;
        let message = ipc.invoke("last_message", json!({ "pane": 543 })).expect("a last message");
        let printed: Value = serde_json::from_str(TURN_OVER).expect("the fixture parses");
        assert_eq!(message["pane"], 543);
        assert_eq!(message["agent"], printed["agent"]);
        assert_eq!(message["text"], printed["text"]);
        assert_eq!(message["turn_over"], true);
    }

    #[rstest]
    #[case::with_its_prefix(NO_PANE)]
    #[case::with_a_trailing_line("Error: there is no pane 543; `cornercase status` lists them\n")]
    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_of_its_one_target(#[case] stderr: &str) {
        let ipc = connected_with(|fake| fake.answer(READ, &[], &[stderr], 1)).await;
        let refused = ipc.invoke("last_message", json!({ "pane": 543 })).expect_err("no such pane");
        assert_eq!(
            refused,
            json!({ "code": "failed", "message": "there is no pane 543; `cornercase status` lists them" })
        );
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_any_other_error_as_it_is() {
        let ipc = connected_with(|fake| fake.fail(READ, Error::Dropped)).await;
        let refused = ipc.invoke("last_message", json!({ "pane": 543 })).expect_err("the read dropped");
        assert_eq!(refused, json!({ "code": "dropped", "message": Error::Dropped.to_string() }));
    }
}

mod without_a_connection {
    use super::*;

    #[test]
    fn says_it_is_not_connected() {
        let ipc = Ipc::new(Studio::new().reloaded());
        let refused = ipc.invoke("last_message", json!({ "pane": 543 })).expect_err("nothing to read from");
        assert_eq!(refused, json!({ "code": "not_connected", "message": "not connected" }));
    }
}
