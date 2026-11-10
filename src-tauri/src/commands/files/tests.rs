use std::time::Duration;

use cornercase_core::cli::files::ReadKind;
use rstest::rstest;
use serde_json::{Value, json};
use tokio::time;

use super::*;
use crate::test_util::{BOOT, Ipc, Streams, Studio, connected_to, connected_with, serving, settle, snapshot, spawned};

const LISTING: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../crates/core/tests/fixtures/control/files/v1/listing.json"));
const TEXT: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../crates/core/tests/fixtures/control/files/v1/text.json"));
const SEARCH: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../crates/core/tests/fixtures/control/files/v1/search.json"));
const LIST: [&str; 8] = ["files", "list", "--workspace", "2", "--json", "--", ".", "./src"];
const READ: [&str; 7] = ["files", "read", "--workspace", "2", "--json", "--", "./src/main.rs"];
const SOUGHT: [&str; 8] = ["files", "search", "--names", "--workspace", "2", "--json", "--", "main"];
const SOUGHT_BEFORE: [&str; 8] = ["files", "search", "--names", "--workspace", "2", "--json", "--", "mai"];

async fn invoked(argv: &[&str], answer: (&str, &str, i32), command: &str, input: Value) -> Result<Value, Value> {
    let (stdout, stderr, code) = answer;
    let stdout: Vec<&str> = stdout.lines().collect();
    let stderr: Vec<&str> = stderr.lines().collect();
    let ipc = connected_with(|fake| fake.answer(argv, &stdout, &stderr, code)).await;
    ipc.invoke(command, input)
}

async fn reading(answer: (&str, &str, i32)) -> Result<Value, Value> {
    invoked(&READ, answer, "files_read", json!({ "workspace": 2, "path": "src/main.rs" })).await
}

fn found() -> Found {
    serde_json::from_str(SEARCH).expect("the fixture parses")
}

fn text(text: &str) -> Read {
    Read { kind: ReadKind::Text, text: Some(text.to_owned()), ..Read::default() }
}

fn missing() -> String {
    json!({ "path": "./src/main.rs", "file": "/work/shop/src/main.rs", "kind": "missing" }).to_string()
}

mod listing {
    use super::*;

    #[rstest]
    #[case::that_all_read(0)]
    #[case::with_a_folder_that_failed(3)]
    #[tokio::test(start_paused = true)]
    async fn answers_each_folder_asked_in_one_call(#[case] code: i32) {
        let args = json!({ "workspace": 2, "paths": ["", "src"] });
        let listed = invoked(&LIST, (LISTING, "", code), "files_list", args).await.expect("a listing");
        let printed: Listing = serde_json::from_str(LISTING).expect("the fixture parses");
        assert_eq!(serde_json::from_value::<Listing>(listed).expect("the answer is a listing"), printed);
    }

    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave_without_json() {
        let args = json!({ "workspace": 2, "paths": ["", "src"] });
        let refused = invoked(&LIST, ("", "Error: no workspace 2\n", 1), "files_list", args).await;
        assert_eq!(refused, Err(json!({ "code": "failed", "message": "no workspace 2" })));
    }
}

mod reading_a_file {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn answers_the_text_cornercase_printed() {
        let shown = reading((TEXT, "", 0)).await.expect("a file");
        let printed: Read = serde_json::from_str(TEXT).expect("the fixture parses");
        assert_eq!(shown, serde_json::to_value(FileShown::Read { read: printed }).expect("a file shown serializes"));
        assert_eq!(shown["shown"], "read");
    }

    #[tokio::test(start_paused = true)]
    async fn answers_a_file_gone_that_exits_with_1() {
        let shown = reading((&missing(), "", 1)).await.expect("a file");
        assert_eq!((&shown["shown"], &shown["read"]["kind"]), (&json!("read"), &json!("missing")));
    }

    #[tokio::test(start_paused = true)]
    async fn says_a_file_past_the_bridge_line_is_too_large_for_the_phone() {
        let refused = reading(("", &format!("{LINE_TOO_LONG}\n"), 2)).await;
        assert_eq!(refused, Err(json!({ "code": "too_large_here", "message": "Too large to show on the phone" })));
    }

    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave_without_json() {
        let refused = reading(("", "Error: src/main.rs is outside the workspace\n", 2)).await;
        assert_eq!(refused, Err(json!({ "code": "failed", "message": "src/main.rs is outside the workspace" })));
    }

    #[tokio::test(start_paused = true)]
    async fn holds_back_a_text_past_the_limit_of_the_phone() {
        let long = format!(r#"{{ "kind": "text", "text": "{}" }}"#, "x".repeat(MOST_BYTES + 1));
        let shown = reading((&long, "", 0)).await.expect("a file");
        assert_eq!(shown, json!({ "shown": "too_large_here", "bytes": MOST_BYTES + 1, "lines": 1 }));
    }
}

mod searching {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn answers_the_names_cornercase_found() {
        let args = json!({ "workspace": 2, "query": "main" });
        let searched = invoked(&SOUGHT, (SEARCH, "", 0), "files_search", args).await.expect("a search");
        assert_eq!(searched, serde_json::to_value(Searched::Found { found: found() }).expect("a search serializes"));
        assert_eq!(searched["searched"], "found");
    }

    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave_without_json() {
        let args = json!({ "workspace": 2, "query": "main" });
        let refused = invoked(&SOUGHT, ("", "Error: no workspace 2\n", 1), "files_search", args).await;
        assert_eq!(refused, Err(json!({ "code": "failed", "message": "no workspace 2" })));
    }

    #[tokio::test(start_paused = true)]
    async fn cancels_the_search_before_and_ends_its_process() {
        let streams = Streams::default();
        let serving = serving(&[snapshot(BOOT, 7, "first")]);
        let lines: Vec<&str> = SEARCH.lines().collect();
        let studio = connected_to(streams.on(serving, &SOUGHT_BEFORE, &[]).answer(&SOUGHT, &lines, &[], 0));
        settle().await;
        let next = async {
            settle().await;
            assert_eq!(streams.running(), 1);
            search(&studio.state, spawned, 2, "main").await
        };
        let both = async { tokio::join!(search(&studio.state, spawned, 2, "mai"), next) };
        let (before, next) = time::timeout(Duration::from_secs(600), both).await.expect("both searches end");
        assert_eq!((before, next), (Ok(Searched::Cancelled), Ok(Searched::Found { found: found() })));
        assert_eq!(streams.running(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn stops_the_search_of_its_workspace_once_the_screen_leaves() {
        let streams = Streams::default();
        let studio = connected_to(streams.on(serving(&[snapshot(BOOT, 7, "first")]), &SOUGHT_BEFORE, &[]));
        settle().await;
        let left = async {
            settle().await;
            stop_search(&studio.state, 3);
            settle().await;
            assert_eq!(streams.running(), 1);
            stop_search(&studio.state, 2);
        };
        let both = async { tokio::join!(search(&studio.state, spawned, 2, "mai"), left) };
        let (before, ()) = time::timeout(Duration::from_secs(600), both).await.expect("the search ends");
        assert_eq!(before, Ok(Searched::Cancelled));
        assert_eq!(streams.running(), 0);
    }

    #[test]
    fn stops_nothing_over_ipc_without_a_connection() {
        let ipc = Ipc::new(Studio::new().reloaded());
        assert_eq!(ipc.invoke("files_search_stop", json!({ "workspace": 2 })), Ok(Value::Null));
    }

    #[tokio::test(start_paused = true)]
    async fn cancels_a_search_whose_machine_went_away() {
        let streams = Streams::default();
        let studio = connected_to(streams.on(serving(&[snapshot(BOOT, 7, "first")]), &SOUGHT_BEFORE, &[]));
        settle().await;
        let gone = async {
            settle().await;
            *studio.state.active() = None;
        };
        let both = async { tokio::join!(search(&studio.state, spawned, 2, "mai"), gone) };
        let (before, ()) = time::timeout(Duration::from_secs(600), both).await.expect("the search ends");
        assert_eq!(before, Ok(Searched::Cancelled));
        assert_eq!(streams.running(), 0);
    }
}

mod the_limit_of_the_phone {
    use super::*;

    #[rstest]
    #[case::bytes_at_the_limit("x".repeat(MOST_BYTES))]
    #[case::lines_at_the_limit("a\n".repeat(MOST_LINES))]
    #[case::empty(String::new())]
    fn shows_a_text_up_to_it(#[case] shown: String) {
        assert_eq!(FileShown::of(text(&shown)), FileShown::Read { read: text(&shown) });
    }

    #[test]
    fn shows_a_read_without_text() {
        let image = Read { kind: ReadKind::Image, ..Read::default() };
        assert_eq!(FileShown::of(image.clone()), FileShown::Read { read: image });
    }

    #[rstest]
    #[case::one_byte_past_it("x".repeat(MOST_BYTES + 1), MOST_BYTES + 1, 1)]
    #[case::one_line_past_it("a\n".repeat(MOST_LINES + 1), 2 * (MOST_LINES + 1), MOST_LINES + 1)]
    fn holds_back_a_text_past_it_with_its_size(#[case] shown: String, #[case] bytes: usize, #[case] lines: usize) {
        assert_eq!(FileShown::of(text(&shown)), FileShown::TooLargeHere { bytes, lines });
    }
}

mod without_a_connection {
    use super::*;

    #[rstest]
    #[case::listing("files_list", json!({ "workspace": 2, "paths": [""] }))]
    #[case::reading_a_file("files_read", json!({ "workspace": 2, "path": "src/main.rs" }))]
    #[case::searching("files_search", json!({ "workspace": 2, "query": "main" }))]
    fn says_it_is_not_connected_rather_than_asking(#[case] command: &str, #[case] args: Value) {
        let ipc = Ipc::new(Studio::new().reloaded());
        let refused = ipc.invoke(command, args).expect_err("no machine");
        assert_eq!(refused, json!({ "code": "not_connected", "message": "no machine is connected" }));
    }
}
