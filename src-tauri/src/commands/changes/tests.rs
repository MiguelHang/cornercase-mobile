use cornercase_core::cli::changes::{Mode, Patch};
use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::commands::error::ErrorCode;
use crate::test_util::{Ipc, Studio, connected_with, queueing, written};

const PATCH: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../crates/core/tests/fixtures/control/changes/v1/patch.json"));
const DONE: &str = r#"{ "status": 0, "text": "" }"#;

fn file_argv(path: &str) -> Vec<String> {
    ["changes", "file", "--workspace", "2", "--mode", "uncommitted", "--json", "--", path].map(str::to_owned).to_vec()
}

fn shown(path: &str) -> Value {
    json!({ "workspace": 2, "path": path, "mode": "uncommitted" })
}

async fn reading(path: &str, stdout: &[&str], stderr: &[&str], code: i32) -> Result<Value, Value> {
    let argv = file_argv(path);
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    let ipc = connected_with(|fake| fake.answer(&argv, stdout, stderr, code)).await;
    ipc.invoke("changes_file", shown(path))
}

fn viewed(viewed: bool, digest: Option<&str>) -> ViewedFile {
    let digest = digest.map(str::to_owned);
    ViewedFile { workspace: 2, path: "src/a.rs".to_owned(), viewed, digest, mode: Mode::Commits }
}

async fn marking(answer: Option<&str>, file: ViewedFile) -> (Result<(), CommandError>, Vec<Value>) {
    let answer = answer.map(|answer| serde_json::from_str(answer).expect("an answer of the queue"));
    let (ipc, lines) = queueing(answer).await;
    let marked = mark_viewed(&ipc.state(), &file).await;
    (marked, written(&lines))
}

mod reading_a_file {
    use super::*;

    #[rstest]
    #[case::plain("src/a.rs")]
    #[case::like_a_flag("--force.rs")]
    #[tokio::test(start_paused = true)]
    async fn answers_the_patch_cornercase_printed_for_the_path(#[case] path: &str) {
        let lines: Vec<&str> = PATCH.lines().collect();
        let patch = reading(path, &lines, &[], 0).await.expect("a patch");
        let printed: Patch = serde_json::from_str(PATCH).expect("the fixture parses");
        assert_eq!(serde_json::from_value::<Patch>(patch).expect("the answer is a patch"), printed);
    }

    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave() {
        let refused = reading("src/a.rs", &[], &["Error: src/a.rs did not change\n"], 1).await;
        assert_eq!(refused, Err(json!({ "code": "failed", "message": "src/a.rs did not change" })));
    }

    #[tokio::test(start_paused = true)]
    async fn says_an_answer_that_is_not_a_patch() {
        let refused = reading("src/a.rs", &["[]"], &[], 0).await.expect_err("not a patch");
        assert_eq!(refused["code"], "json");
    }
}

mod marking_viewed {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn queues_the_digest_of_the_version_shown_without_the_mode() {
        let (marked, written) = marking(Some(DONE), viewed(true, Some("00c0ffee12345678"))).await;
        let args = ["changes", "viewed", "--workspace", "2", "--digest=00c0ffee12345678", "--", "src/a.rs"];
        assert_eq!((marked, written), (Ok(()), vec![json!({ "args": args })]));
    }

    #[rstest]
    #[case::without_a_digest(None)]
    #[case::with_a_digest(Some("00c0ffee12345678"))]
    #[tokio::test(start_paused = true)]
    async fn clears_the_mark_in_the_mode_shown_and_never_sends_a_digest(#[case] digest: Option<&str>) {
        let (marked, written) = marking(Some(DONE), viewed(false, digest)).await;
        let args = ["changes", "viewed", "--workspace", "2", "--clear", "--mode", "commits", "--", "src/a.rs"];
        assert_eq!((marked, written), (Ok(()), vec![json!({ "args": args })]));
    }

    #[rstest]
    #[case::none(None)]
    #[case::empty(Some(""))]
    #[tokio::test(start_paused = true)]
    async fn refuses_a_mark_without_the_digest_and_queues_nothing(#[case] digest: Option<&str>) {
        let (marked, written) = marking(Some(DONE), viewed(true, digest)).await;
        assert_eq!((marked, written), (Err(CommandError::no_digest()), Vec::new()));
    }

    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave() {
        let refusal = r#"{ "status": 1, "error": "Error: the digest names no version of src/a.rs\n" }"#;
        let (marked, _) = marking(Some(refusal), viewed(true, Some("00c0ffee12345678"))).await;
        let reason = "the digest names no version of src/a.rs".to_owned();
        assert_eq!(marked, Err(CommandError { code: ErrorCode::Failed, message: reason }));
    }

    #[tokio::test(start_paused = true)]
    async fn may_have_marked_it_when_the_answer_is_lost() {
        let (marked, written) = marking(None, viewed(true, Some("00c0ffee12345678"))).await;
        assert_eq!((marked, written.len()), (Err(CommandError::maybe_sent()), 1));
    }
}

mod without_a_connection {
    use super::*;

    #[rstest]
    #[case::reading_a_file("changes_file", shown("src/a.rs"))]
    #[case::marking_viewed("changes_viewed", json!({ "file": { "workspace": 2, "path": "src/a.rs", "viewed": false, "digest": null, "mode": "all" } }))]
    fn says_it_is_not_connected_rather_than_asking(#[case] command: &str, #[case] args: Value) {
        let ipc = Ipc::new(Studio::new().reloaded());
        let refused = ipc.invoke(command, args).expect_err("no machine");
        assert_eq!(refused, json!({ "code": "not_connected", "message": "no machine is connected" }));
    }
}
