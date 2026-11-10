use cornercase_core::error::Error;
use rstest::rstest;
use serde_json::{Value, json};

use super::{mark_pane_reviewed, press_key, send_message};
use crate::commands::error::CommandError;
use crate::test_util::{Ipc, Studio, connected_with, queueing, written};

const READ: &[&str] = &["read", "--last-message", "--pane", "543", "--json"];
const TURN_OVER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/last-message/0.13.19/turn-over.json"
));
const NO_PANE: &str = "Error: there is no pane 543; `cornercase status` lists them";
const SEND: [&str; 7] = ["send", "--pane", "543", "--enter", "--timeout", "30", "-"];
const FORCED: [&str; 8] = ["send", "--pane", "543", "--force", "--enter", "--timeout", "30", "-"];

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

async fn sending(answer: Option<Value>, text: &str, force: bool) -> (Result<(), Value>, Vec<Value>) {
    let (ipc, lines) = queueing(answer).await;
    let sent = send_message(&ipc.state(), 543, text.to_owned(), force).await.map_err(|error| json_of(&error));
    (sent, written(&lines))
}

async fn pressing(answer: Option<Value>, key: &str) -> (Result<(), Value>, Vec<Value>) {
    let (ipc, lines) = queueing(answer).await;
    let pressed = press_key(&ipc.state(), 543, key).await.map_err(|error| json_of(&error));
    (pressed, written(&lines))
}

async fn reviewing(answer: Option<Value>) -> (Result<(), Value>, Vec<Value>) {
    let (ipc, lines) = queueing(answer).await;
    let marked = mark_pane_reviewed(&ipc.state(), 543).await.map_err(|error| json_of(&error));
    (marked, written(&lines))
}

pub(super) fn json_of(error: &CommandError) -> Value {
    serde_json::to_value(error).expect("an error serializes")
}

mod sending_a_message {
    use super::*;

    const WAITING: &str = "the agent in pane 543 is waiting for an answer to a question or a permission prompt, and \
                           what you send would answer it; ask the user, then answer with `cornercase keys`";
    const NOT_CONFIRMED: &str = "not confirmed: the prompt may not have been submitted: the agent in pane 543 \
                                 recorded no new prompt after the Enter; `cornercase read --pane 543` shows where it \
                                 is, read it before sending again";

    #[rstest]
    #[case::plain("fix the failing test")]
    #[case::like_a_flag("--help me with the build")]
    #[case::on_several_lines("first line\nsecond line")]
    #[tokio::test(start_paused = true)]
    async fn writes_the_text_on_stdin_and_never_in_the_args(#[case] text: &str) {
        let (sent, written) = sending(Some(json!({ "status": 0, "text": "" })), text, false).await;
        assert_eq!(sent, Ok(()));
        assert_eq!(written, [json!({ "args": SEND, "stdin": text })]);
    }

    #[tokio::test(start_paused = true)]
    async fn forces_it_past_a_held_draft_when_asked() {
        let (sent, written) = sending(Some(json!({ "status": 0, "text": "" })), "what is in it?", true).await;
        assert_eq!(sent, Ok(()));
        assert_eq!(written, [json!({ "args": FORCED, "stdin": "what is in it?" })]);
    }

    #[tokio::test(start_paused = true)]
    async fn presses_enter_when_forced_with_nothing_to_add() {
        let (sent, written) = sending(Some(json!({ "status": 0, "text": "" })), "", true).await;
        assert_eq!(sent, Ok(()));
        assert_eq!(written, [json!({ "args": ["keys", "--pane", "543", "enter"] })]);
    }

    #[rstest]
    #[case::waiting(WAITING)]
    #[case::not_confirmed(NOT_CONFIRMED)]
    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave(#[case] reason: &str) {
        let refusal = json!({ "status": 1, "error": format!("Error: {reason}\n") });
        let (sent, _) = sending(Some(refusal), "fix the failing test", false).await;
        assert_eq!(sent, Err(json!({ "code": "failed", "message": reason })));
    }

    #[tokio::test(start_paused = true)]
    async fn may_have_sent_it_when_the_answer_is_lost() {
        let (sent, written) = sending(None, "fix the failing test", false).await;
        assert_eq!(written.len(), 1);
        assert_eq!(sent, Err(json_of(&CommandError::maybe_sent())));
    }
}

mod pressing_a_key {
    use super::*;

    const NOT_WAITING: &str = "the agent in pane 543 is not waiting for an answer";

    #[rstest]
    #[case::down("down")]
    #[case::enter("enter")]
    #[case::interrupt("ctrl+c")]
    #[case::a_digit("1")]
    #[tokio::test(start_paused = true)]
    async fn queues_the_keys_command_without_stdin(#[case] key: &str) {
        let (pressed, written) = pressing(Some(json!({ "status": 0, "text": "" })), key).await;
        assert_eq!(pressed, Ok(()));
        assert_eq!(written, [json!({ "args": ["keys", "--pane", "543", key] })]);
    }

    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave() {
        let refusal = json!({ "status": 1, "error": format!("Error: {NOT_WAITING}\n") });
        let (pressed, _) = pressing(Some(refusal), "enter").await;
        assert_eq!(pressed, Err(json!({ "code": "failed", "message": NOT_WAITING })));
    }

    #[tokio::test(start_paused = true)]
    async fn may_have_sent_it_when_the_answer_is_lost() {
        let (pressed, written) = pressing(None, "enter").await;
        assert_eq!(written.len(), 1);
        assert_eq!(pressed, Err(json_of(&CommandError::maybe_sent())));
    }

    #[rstest]
    #[case::empty("")]
    #[case::an_option("--force")]
    #[case::two_keys("down enter")]
    #[tokio::test(start_paused = true)]
    async fn refuses_a_name_the_app_never_sends_and_queues_nothing(#[case] key: &str) {
        let (pressed, written) = pressing(Some(json!({ "status": 0, "text": "" })), key).await;
        assert_eq!(pressed, Err(json_of(&CommandError::from(Error::UnknownKey(key.to_owned())))));
        assert_eq!(written, Vec::<Value>::new());
    }
}

mod marking_it_reviewed {
    use super::*;

    const NO_AGENT: &str = "there is no agent in pane 543";

    #[tokio::test(start_paused = true)]
    async fn queues_the_mark_command_without_stdin() {
        let (marked, written) = reviewing(Some(json!({ "status": 0, "text": "" }))).await;
        assert_eq!(marked, Ok(()));
        assert_eq!(written, [json!({ "args": ["mark", "--pane", "543", "--reviewed"] })]);
    }

    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave() {
        let refusal = json!({ "status": 1, "error": format!("Error: {NO_AGENT}\n") });
        let (marked, _) = reviewing(Some(refusal)).await;
        assert_eq!(marked, Err(json!({ "code": "failed", "message": NO_AGENT })));
    }

    #[tokio::test(start_paused = true)]
    async fn may_have_sent_it_when_the_answer_is_lost() {
        let (marked, written) = reviewing(None).await;
        assert_eq!(written.len(), 1);
        assert_eq!(marked, Err(json_of(&CommandError::maybe_sent())));
    }
}

mod without_a_connection {
    use super::*;

    #[test]
    fn says_it_is_not_connected() {
        let ipc = Ipc::new(Studio::new().reloaded());
        let refused = ipc.invoke("last_message", json!({ "pane": 543 })).expect_err("nothing to read from");
        assert_eq!(refused, json!({ "code": "not_connected", "message": "no machine is connected" }));
    }

    #[rstest]
    #[case::sending("send", json!({ "pane": 543, "text": "hello", "force": false }))]
    #[case::attaching("attach", json!({ "pane": 543 }))]
    #[case::pressing_a_key("keys", json!({ "pane": 543, "key": "enter" }))]
    #[case::reviewing("mark_reviewed", json!({ "pane": 543 }))]
    fn says_it_is_not_connected_rather_than_queueing(#[case] command: &str, #[case] args: Value) {
        let ipc = Ipc::new(Studio::new().reloaded());
        let refused = ipc.invoke(command, args).expect_err("nowhere to queue");
        assert_eq!(refused, json!({ "code": "not_connected", "message": "no machine is connected" }));
    }
}
