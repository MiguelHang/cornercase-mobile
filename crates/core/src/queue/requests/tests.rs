use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::transport::fake::FakeTransport;
use crate::transport::output;

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|&word| word.to_owned()).collect()
}

fn sent(line: &str) -> Value {
    serde_json::from_str(line).expect("the line is JSON")
}

mod line {
    use super::*;

    #[test]
    fn is_the_command_line_after_cornercase() {
        let line = line(&words(&["focus", "--tab", "6", "--view", "21"]), None);
        assert_eq!(sent(&line), json!({ "args": ["focus", "--tab", "6", "--view", "21"] }));
    }

    #[test]
    fn carries_what_the_command_reads_from_its_input() {
        let line = line(&words(&["send", "--pane", "5", "-"]), Some("cargo test"));
        assert_eq!(sent(&line), json!({ "args": ["send", "--pane", "5", "-"], "stdin": "cargo test" }));
    }

    #[test]
    fn stays_one_line_whatever_its_input_holds() {
        let line = line(&words(&["todo", "add", "-"]), Some("two\nlines\r\n"));
        assert!(!line.contains('\n'));
        assert_eq!(sent(&line)["stdin"], "two\nlines\r\n");
    }
}

mod answer {
    use super::*;

    #[rstest]
    #[case::json_and_text(r#"{"status":0,"json":{"pane":5},"text":"moved\n"}"#, "{\"pane\":5}\nmoved")]
    #[case::only_json(r#"{"status":0,"json":{"pane":5,"tab":6}}"#, "{\"pane\":5,\"tab\":6}")]
    #[case::only_text(r#"{"status":0,"text":"project 4\n"}"#, "project 4")]
    #[case::nothing(r#"{"status":0}"#, "")]
    #[case::no_status(r#"{"text":"ok"}"#, "ok")]
    #[case::fields_it_does_not_know(r#"{"status":0,"text":"ok\n","took":12,"warnings":[]}"#, "ok")]
    fn gives_what_the_command_printed(#[case] line: &str, #[case] printed: &str) {
        assert_eq!(answer(line), Ok(printed.to_owned()));
    }

    #[rstest]
    #[case::one_line(&["project 4"])]
    #[case::several_lines(&["a", "b"])]
    #[case::empty_lines(&["", "a", ""])]
    #[case::a_carriage_return(&["a\r", "b"])]
    #[tokio::test]
    async fn prints_as_the_single_command_does(#[case] lines: &[&str]) {
        let fake = FakeTransport::default().answer(&["status"], lines, &[], 0);
        let single = output(&fake, words(&["status"])).await.expect("an answer");
        let text: String = lines.iter().flat_map(|&line| [line, "\n"]).collect();
        let line = json!({ "status": 0, "text": text }).to_string();
        assert_eq!(answer(&line), Ok(single.stdout));
    }

    #[rstest]
    #[case::its_error(r#"{"status":1,"error":"there is no tab 9\n","stderr":"warning"}"#, 1, "there is no tab 9")]
    #[case::its_warnings(r#"{"status":3,"stderr":"warning: pane 7 is gone\n"}"#, 3, "warning: pane 7 is gone")]
    #[case::nothing_said(r#"{"status":2}"#, 2, "")]
    fn gives_a_failure_with_what_it_said(#[case] line: &str, #[case] code: i32, #[case] stderr: &str) {
        assert_eq!(answer(line), Err(Error::Failed { code, stderr: stderr.to_owned() }));
    }

    #[test]
    fn says_the_server_restarted_on_status_4() {
        let line = r#"{"status":4,"error":"the boot changed"}"#;
        assert_eq!(answer(line), Err(Error::StaleBoot("the boot changed".to_owned())));
    }

    #[rstest]
    #[case::not_json("oops secret", "not JSON at line 1 column 1")]
    #[case::cut_short(r#"{"status":0,"text":"sec"#, "the JSON ends too early at line 1 column 23")]
    #[case::not_an_object(r#""secret""#, "not the expected shape at line 1 column 8")]
    fn names_what_is_wrong_with_a_line_without_its_text(#[case] line: &str, #[case] reason: &str) {
        let error = answer(line).expect_err("not an answer");
        assert_eq!(error, Error::Json(reason.to_owned()));
        assert!(!error.to_string().contains("secret"));
    }
}

mod absent {
    use super::*;

    fn exit(code: i32, stderr: &str) -> Exit {
        Exit { code, stderr: stderr.to_owned() }
    }

    #[rstest]
    #[case::no_requests_command(exit(2, "error: unrecognized subcommand 'requests'\n\nUsage: cornercase [OPTIONS]"))]
    #[case::no_follow(exit(2, "error: unexpected argument '--follow' found\n\nUsage: cornercase requests"))]
    fn is_a_cli_that_does_not_know_the_command(#[case] exit: Exit) {
        assert!(absent(&exit));
    }

    #[rstest]
    #[case::another_usage_error(exit(2, "the bridge does not allow `kill-server`"))]
    #[case::another_code(exit(1, "error: unrecognized subcommand 'requests'"))]
    #[case::a_dropped_server(exit(1, "Error: the connection to cornercase dropped"))]
    #[case::a_clean_end(exit(0, ""))]
    fn is_not_any_other_end(#[case] exit: Exit) {
        assert!(!absent(&exit));
    }
}
