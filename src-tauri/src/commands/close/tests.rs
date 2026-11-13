use cornercase_core::cli::close::AgentClosing;
use rstest::rstest;
use serde_json::{Value, json};

use super::{close_agent, close_agent_check, closing};
use crate::commands::error::{CommandError, ErrorCode};
use crate::test_util::{BOOT, queueing, written};

const CHECK_SUBAGENT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/close-agent/0.13.31/check-subagent.json"
));
const CLOSED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/close-agent/0.13.31/closed.json"
));
const CHECK: [&str; 8] = ["close", "--agent", "--pane", "10", "--boot", BOOT, "--check", "--json"];
const CLOSE: [&str; 7] = ["close", "--agent", "--pane", "10", "--boot", BOOT, "--json"];
const NEVER: [&str; 4] = ["--remove-worktree", "--delete-branch", "--force", "--unlock"];

fn printed(fixture: &str) -> Value {
    json!({ "status": 0, "text": "", "json": serde_json::from_str::<Value>(fixture).expect("the fixture parses") })
}

fn read(fixture: &str) -> AgentClosing {
    serde_json::from_str(fixture).expect("the fixture parses")
}

async fn closed(answer: Option<Value>, check: bool) -> (Result<AgentClosing, CommandError>, Vec<Value>) {
    let (ipc, lines) = queueing(answer).await;
    let closed = closing(&ipc.state(), 10, BOOT, check).await;
    (closed, written(&lines))
}

mod checking {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn queues_the_check_with_the_boot_of_the_row() {
        let (checked, written) = closed(Some(printed(CHECK_SUBAGENT)), true).await;
        assert_eq!(written, [json!({ "args": CHECK })]);
        assert_eq!(checked, Ok(read(CHECK_SUBAGENT)));
        assert!(!read(CHECK_SUBAGENT).closed);
    }
}

mod closing_the_agent {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn queues_the_close_without_the_check() {
        let (closed, written) = closed(Some(printed(CLOSED)), false).await;
        assert_eq!(written, [json!({ "args": CLOSE })]);
        assert_eq!(closed, Ok(read(CLOSED)));
        assert!(read(CLOSED).closed);
    }

    #[rstest]
    #[case::check(true)]
    #[case::close(false)]
    #[tokio::test(start_paused = true)]
    async fn never_asks_to_delete_anything(#[case] check: bool) {
        let (_, written) = closed(Some(printed(CLOSED)), check).await;
        let args = written[0]["args"].as_array().expect("the args");
        assert!(NEVER.iter().all(|flag| !args.contains(&json!(flag))));
    }
}

mod a_failure {
    use super::*;

    const NO_AGENT: &str = "pane 10 runs no agent cornercase reads";

    #[rstest]
    #[case::check(true)]
    #[case::close(false)]
    #[tokio::test(start_paused = true)]
    async fn says_the_server_restarted_on_exit_4(#[case] check: bool) {
        let stale = json!({ "status": 4, "error": "Error: stale boot: the server restarted\n" });
        let (refused, _) = closed(Some(stale), check).await;
        assert_eq!(refused.expect_err("a stale boot").code, ErrorCode::StaleBoot);
    }

    #[tokio::test(start_paused = true)]
    async fn gives_the_reason_cornercase_gave() {
        let refusal = json!({ "status": 1, "error": format!("Error: {NO_AGENT}\n") });
        let (refused, _) = closed(Some(refusal), true).await;
        assert_eq!(refused, Err(CommandError { code: ErrorCode::Failed, message: NO_AGENT.to_owned() }));
    }

    #[tokio::test(start_paused = true)]
    async fn may_have_closed_it_when_the_answer_is_lost() {
        let (refused, written) = closed(None, false).await;
        assert_eq!(written.len(), 1);
        assert_eq!(refused, Err(CommandError::maybe_sent()));
    }

    #[tokio::test(start_paused = true)]
    async fn says_why_an_answer_does_not_read() {
        let (refused, _) = closed(Some(json!({ "status": 0, "text": "closed" })), false).await;
        let refused = refused.expect_err("an unreadable answer");
        assert_eq!(refused.code, ErrorCode::Json);
        assert!(refused.message.contains("line 1"));
    }
}

mod the_commands {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn check_queues_the_check_and_returns_what_cornercase_answered() {
        let (ipc, lines) = queueing(Some(printed(CHECK_SUBAGENT))).await;
        let checked = close_agent_check(ipc.state(), 10, BOOT.to_owned()).await;
        assert_eq!(written(&lines), [json!({ "args": CHECK })]);
        assert_eq!(checked, Ok(read(CHECK_SUBAGENT)));
    }

    #[tokio::test(start_paused = true)]
    async fn close_queues_the_close_and_returns_what_cornercase_answered() {
        let (ipc, lines) = queueing(Some(printed(CLOSED))).await;
        let closed = close_agent(ipc.state(), 10, BOOT.to_owned()).await;
        assert_eq!(written(&lines), [json!({ "args": CLOSE })]);
        assert_eq!(closed, Ok(read(CLOSED)));
    }
}
