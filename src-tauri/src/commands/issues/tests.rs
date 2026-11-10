use rstest::rstest;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::test_util::{Ipc, Studio, connected_with};

const CONNECTED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/issue-sources/v1/connected.json"
));
const PARTIAL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/issue-list/v1/partial.json"
));
const COMMENTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/issue-read/v1/comments.json"
));
const SOURCES: [&str; 5] = ["issue", "sources", "--project", "7", "--json"];
const LIST: [&str; 5] = ["issue", "list", "--project", "7", "--json"];
const MINE: [&str; 9] =
    ["issue", "list", "--project", "7", "--source=github", "--closed", "--assignee", "me", "--json"];
const READ: [&str; 8] = ["issue", "read", "--project", "7", "--source=github", "--json", "--", "#41"];

async fn invoked(argv: &[&str], answer: (&str, &str, i32), command: &str, input: Value) -> Result<Value, Value> {
    let (stdout, stderr, code) = answer;
    let stdout: Vec<&str> = stdout.lines().collect();
    let stderr: Vec<&str> = stderr.lines().collect();
    let ipc = connected_with(|fake| fake.answer(argv, &stdout, &stderr, code)).await;
    ipc.invoke(command, input)
}

fn printed<T: DeserializeOwned + serde::Serialize>(fixture: &str) -> Value {
    let read: T = serde_json::from_str(fixture).expect("the fixture parses");
    serde_json::to_value(read).expect("the fixture serializes")
}

async fn listing(argv: &[&str], answer: (&str, &str, i32), filter: Value) -> Result<Value, Value> {
    invoked(argv, answer, "issue_list", json!({ "project": 7, "filter": filter })).await
}

async fn reading(answer: (&str, &str, i32)) -> Result<Value, Value> {
    invoked(&READ, answer, "issue_read", json!({ "project": 7, "source": "github", "key": "#41" })).await
}

mod the_trackers {
    use cornercase_core::cli::issues::Sources;

    use super::*;

    #[tokio::test(start_paused = true)]
    async fn answers_each_one_with_its_state() {
        let read = invoked(&SOURCES, (CONNECTED, "", 0), "issue_sources", json!({ "project": 7 })).await;
        assert_eq!(read, Ok(printed::<Sources>(CONNECTED)));
    }

    #[tokio::test(start_paused = true)]
    async fn fail_with_the_reason_cornercase_gave_without_json() {
        let refused =
            invoked(&SOURCES, ("", "Error: no project 7\n", 1), "issue_sources", json!({ "project": 7 })).await;
        assert_eq!(refused, Err(json!({ "code": "failed", "message": "no project 7" })));
    }
}

mod a_list {
    use cornercase_core::cli::issues::Lists;

    use super::*;

    #[rstest]
    #[case::of_every_tracker(0)]
    #[case::with_a_tracker_that_failed(3)]
    #[tokio::test(start_paused = true)]
    async fn answers_what_the_trackers_listed(#[case] code: i32) {
        let failed = "Shortcut: Shortcut rejected the token\n";
        let listed = listing(&LIST, (PARTIAL, failed, code), json!({})).await.expect("a list");
        assert_eq!(listed, printed::<Lists>(PARTIAL));
        assert_eq!(listed["sources"][1]["error"], "Shortcut rejected the token");
    }

    #[tokio::test(start_paused = true)]
    async fn sends_the_filters_the_phone_chose() {
        let filter = json!({ "source": "github", "closed": true, "mine": true, "cached": false });
        let listed = listing(&MINE, (PARTIAL, "", 0), filter).await.expect("a list");
        assert_eq!(listed["issues"][0]["key"], "#41");
    }

    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave_without_json() {
        let refused = listing(&LIST, ("", "Error: no project 7\n", 1), json!({})).await;
        assert_eq!(refused, Err(json!({ "code": "failed", "message": "no project 7" })));
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_server_restarted_even_with_a_list_printed() {
        let refused = listing(&LIST, (PARTIAL, "Error: the server restarted\n", 4), json!({})).await;
        assert_eq!(refused.expect_err("a stale boot")["code"], "stale_boot");
    }
}

mod an_issue {
    use cornercase_core::cli::issues::ReadIssue;

    use super::*;

    #[tokio::test(start_paused = true)]
    async fn answers_its_body_comments_and_prompt() {
        let read = reading((COMMENTS, "", 0)).await.expect("an issue");
        assert_eq!(read, printed::<ReadIssue>(COMMENTS));
        assert_eq!(read["prompt"], "https://github.com/acme/shop/issues/41");
    }

    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave_without_json() {
        let refused = reading(("", "Error: GitHub has no issue #41\n", 1)).await;
        assert_eq!(refused, Err(json!({ "code": "failed", "message": "GitHub has no issue #41" })));
    }
}

mod over_ipc {
    use super::*;

    #[rstest]
    #[case::the_trackers("issue_sources", json!({ "project": 7 }))]
    #[case::a_list("issue_list", json!({ "project": 7, "filter": {} }))]
    #[case::an_issue("issue_read", json!({ "project": 7, "source": "github", "key": "#41" }))]
    fn refuses_without_a_connected_machine(#[case] command: &str, #[case] input: Value) {
        let ipc = Ipc::new(Studio::new().reloaded());
        let refused = ipc.invoke(command, input).expect_err("no machine");
        assert_eq!(refused["code"], "not_connected");
    }
}
