use cornercase_core::cli::changes::{Mode, Patch, Summary};
use cornercase_core::transport::fake::FakeTransport;
use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::commands::error::ErrorCode;
use crate::test_util::{
    BOOT, Ipc, Recorder, Streams, Studio, connected_to, connected_with, queueing, serving, settle, spawned, written,
};

const PATCH: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../crates/core/tests/fixtures/control/changes/v1/patch.json"));
const SUMMARY: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../crates/core/tests/fixtures/control/changes/v1/summary.json"));
const LISTED: Changes = Changes { workspace: 2, mode: Mode::Commits };
const FOLLOWED: [Changes; 3] =
    [LISTED, Changes { workspace: 2, mode: Mode::All }, Changes { workspace: 3, mode: Mode::Commits }];
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

fn summary() -> Summary {
    serde_json::from_str(SUMMARY).expect("the summary fixture parses")
}

fn with_workspaces(ids: &[u64]) -> String {
    let workspaces: Vec<Value> = ids.iter().map(|id| json!({ "id": id })).collect();
    json!({ "boot": BOOT, "revision": 7, "version": "first", "projects": [{ "workspaces": workspaces }] }).to_string()
}

fn listing(streams: &Streams, ids: &[u64], line: &str) -> FakeTransport {
    FOLLOWED.iter().fold(serving(&[with_workspaces(ids)]), |fake, changes| {
        let argv = changes::follow_args(changes.workspace, changes.mode);
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        streams.on(fake, &argv, &[line])
    })
}

struct Followed {
    studio: Studio,
    streams: Streams,
    recorder: Recorder,
}

impl Followed {
    async fn printing(ids: &[u64], line: &str) -> Self {
        let streams = Streams::default();
        let studio = connected_to(listing(&streams, ids, line));
        let recorder = Recorder::default();
        follow(&studio.state, recorder.clone(), spawned, LISTED).expect("the changes are followed");
        settle().await;
        Self { studio, streams, recorder }
    }

    async fn of(ids: &[u64]) -> Self {
        Self::printing(ids, &summary_line()).await
    }
}

fn summary_line() -> String {
    serde_json::to_string(&summary()).expect("a summary serializes")
}

fn listed(machine: u64, summary: Option<Summary>, closed: bool) -> ChangesChanged {
    ChangesChanged { machine, workspace: 2, mode: Mode::Commits, summary, closed }
}

mod following {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn needs_a_connected_machine() {
        let studio = Studio::new();
        let refused = follow(&studio.state, Recorder::default(), spawned, LISTED).expect_err("no machine");
        assert_eq!(refused, CommandError::not_connected());
    }

    #[tokio::test(start_paused = true)]
    async fn hands_each_summary_of_the_workspace_and_mode_as_changes_changed() {
        let followed = Followed::of(&[2]).await;
        let machine = followed.studio.id.0;
        assert_eq!(followed.recorder.changes(), [listed(machine, Some(summary()), false)]);
        assert_eq!(followed.streams.running(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_workspace_closed_when_the_session_lacks_it() {
        let followed = Followed::of(&[3]).await;
        assert_eq!(followed.recorder.changes(), [listed(followed.studio.id.0, None, true)]);
        assert_eq!(followed.streams.running(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn tells_an_unreadable_line_as_a_toast_and_keeps_following() {
        let followed = Followed::printing(&[2], "{\"boot\":").await;
        let emitted = followed.recorder.all();
        assert!(matches!(&emitted[..], [Emitted::Toast(_)]), "{emitted:?}");
        assert_eq!(followed.streams.running(), 1);
    }

    #[rstest]
    #[case::the_same(FOLLOWED[0])]
    #[case::in_another_mode(FOLLOWED[1])]
    #[case::of_another_workspace(FOLLOWED[2])]
    #[tokio::test(start_paused = true)]
    async fn again_stops_the_list_it_followed_before(#[case] next: Changes) {
        let followed = Followed::of(&[2, 3]).await;
        follow(&followed.studio.state, Recorder::default(), spawned, next).expect("followed again");
        settle().await;
        assert_eq!(followed.streams.running(), 1);
    }
}

mod unfollowing {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn the_workspace_cuts_the_list() {
        let followed = Followed::of(&[2]).await;
        unfollow(&followed.studio.state, LISTED.workspace);
        settle().await;
        assert_eq!(followed.streams.running(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn another_workspace_leaves_the_list() {
        let followed = Followed::of(&[2]).await;
        unfollow(&followed.studio.state, 3);
        settle().await;
        assert_eq!(followed.streams.running(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn the_command_cuts_the_list_of_its_workspace_over_ipc() {
        let followed = Followed::of(&[2]).await;
        let ipc = Ipc::new(followed.studio.state);
        assert_eq!(ipc.invoke("changes_unfollow", json!({ "workspace": LISTED.workspace })), Ok(Value::Null));
        settle().await;
        assert_eq!(followed.streams.running(), 0);
    }

    #[test]
    fn nothing_over_ipc_without_a_follow() {
        let ipc = Ipc::new(Studio::new().reloaded());
        assert_eq!(ipc.invoke("changes_unfollow", json!({ "workspace": 2 })), Ok(Value::Null));
    }
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
    #[case::following("changes_follow", json!({ "workspace": 2, "mode": "commits" }))]
    #[case::marking_viewed("changes_viewed", json!({ "file": { "workspace": 2, "path": "src/a.rs", "viewed": false, "digest": null, "mode": "all" } }))]
    fn says_it_is_not_connected_rather_than_asking(#[case] command: &str, #[case] args: Value) {
        let ipc = Ipc::new(Studio::new().reloaded());
        let refused = ipc.invoke(command, args).expect_err("no machine");
        assert_eq!(refused, json!({ "code": "not_connected", "message": "no machine is connected" }));
    }
}
