use std::future::Future;

use futures_util::stream::BoxStream;
use futures_util::{FutureExt, SinkExt, StreamExt};
use rstest::rstest;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use super::*;
use crate::transport::fake::FakeTransport;
use crate::transport::{Closed, Exit, Out, OutSink, Stdin, output};

const NO_REQUESTS_COMMAND: &str = "error: unrecognized subcommand 'requests'\n\nUsage: cornercase [OPTIONS]";
const NO_FOLLOW_FLAG: &str = "error: unexpected argument '--follow' found\n\nUsage: cornercase requests";

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|&word| word.to_owned()).collect()
}

fn sent(line: &str) -> Value {
    serde_json::from_str(line).expect("the line is JSON")
}

fn exit(code: i32, stderr: &str) -> Exit {
    Exit { code, stderr: stderr.to_owned() }
}

fn follow<S, F>(script: S) -> Requests
where
    S: Fn(BoxStream<'static, Stdin>, OutSink) -> F + Send + Sync + 'static,
    F: Future<Output = End> + Send + 'static,
{
    Requests::start(Arc::new(FakeTransport::default().script(&FOLLOW, script)))
}

fn echoing(seen: mpsc::UnboundedSender<String>) -> Requests {
    follow(move |mut stdin, mut out| {
        let seen = seen.clone();
        async move {
            while let Some(Stdin::Text(line)) = stdin.next().await {
                seen.send(line.clone()).expect("the test reads what was written");
                say(&mut out, &line).await?;
            }
            Ok(exit(0, ""))
        }
    })
}

#[derive(Debug, Clone, Copy)]
enum Then {
    Ends,
    EndsLater,
    Reads,
}

fn answering_once(end: End, then: Then) -> Requests {
    follow(move |mut stdin, mut out| {
        let end = end.clone();
        async move {
            let Some(Stdin::Text(line)) = stdin.next().await else { return end };
            say(&mut out, &line).await?;
            match then {
                Then::Ends => {}
                Then::EndsLater => tokio::task::yield_now().await,
                Then::Reads => drop(stdin.next().await),
            }
            end
        }
    })
}

fn ending(end: End, reads: bool) -> Requests {
    follow(move |mut stdin, _| {
        let end = end.clone();
        async move {
            if reads {
                stdin.next().await;
            }
            end
        }
    })
}

async fn say(out: &mut OutSink, line: &str) -> Result<(), Error> {
    let args = serde_json::from_str::<Value>(line).map_or(Value::Null, |line| line["args"].clone());
    let answer = match args[0].as_str() {
        Some("fail") => json!({ "status": 1, "error": "it failed" }),
        _ => json!({ "status": 0, "json": args }),
    };
    out.send(Out::Stdout(answer.to_string())).await.map_err(|Closed| Error::OutputClosed)
}

fn answered(json: &str) -> Asked {
    Asked::Answered(Ok(json.to_owned()))
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

mod ask {
    use super::*;

    #[tokio::test]
    async fn writes_each_line_once_the_one_before_is_answered() {
        let (seen, mut written) = mpsc::unbounded_channel();
        let mut requests = echoing(seen);
        let focus = line(&words(&["focus", "--tab", "6"]), None);
        let send = line(&words(&["send", "--pane", "5", "-"]), Some("ls"));
        assert_eq!(requests.ask(&focus).await, answered(r#"["focus","--tab","6"]"#));
        assert_eq!(written.try_recv(), Ok(format!("{focus}\n")));
        assert!(written.try_recv().is_err());
        assert_eq!(requests.ask(&send).await, answered(r#"["send","--pane","5","-"]"#));
        assert_eq!(written.try_recv(), Ok(format!("{send}\n")));
    }

    #[tokio::test]
    async fn goes_on_after_a_command_that_failed() {
        let (seen, _written) = mpsc::unbounded_channel();
        let mut requests = echoing(seen);
        let failed = Asked::Answered(Err(Error::Failed { code: 1, stderr: "it failed".to_owned() }));
        assert_eq!(requests.ask(&line(&words(&["fail"]), None)).await, failed);
        assert_eq!(requests.ask(&line(&words(&["status"]), None)).await, answered(r#"["status"]"#));
    }

    fn crashed(code: i32, why: &str) -> Error {
        Error::Failed { code, stderr: why.to_owned() }
    }

    #[rstest]
    #[case::an_exit(Ok(exit(1, "Error: the server stopped")), crashed(1, "Error: the server stopped"))]
    #[case::a_clean_exit(Ok(exit(0, "")), crashed(0, ENDED))]
    #[case::a_dropped_connection(Err(Error::Dropped), Error::Dropped)]
    #[case::an_old_cli_message_after_an_answer(
        Ok(exit(2, NO_REQUESTS_COMMAND)),
        crashed(2, "Usage: cornercase [OPTIONS]")
    )]
    #[tokio::test]
    async fn says_the_answer_was_lost_and_why_when_the_process_ends_with_a_line_written(
        #[case] end: End,
        #[case] why: Error,
    ) {
        let mut requests = answering_once(end, Then::Reads);
        assert_eq!(requests.ask(&line(&words(&["status"]), None)).await, answered(r#"["status"]"#));
        let unanswered = requests.ask(&line(&words(&["focus", "--pane", "5"]), None)).await;
        assert_eq!(unanswered, Asked::Answered(Err(Error::Unanswered)));
        assert_eq!(requests.ended().await, Some(why));
    }

    #[tokio::test]
    async fn says_why_the_process_ended_before_its_first_answer() {
        let no_server = "Error: no cornercase server is running";
        let mut requests = ending(Ok(exit(1, no_server)), true);
        let unanswered = requests.ask(&line(&words(&["status"]), None)).await;
        assert_eq!(unanswered, Asked::Answered(Err(Error::Unanswered)));
        assert_eq!(requests.ended().await, Some(crashed(1, no_server)));
    }

    #[tokio::test]
    async fn leaves_the_line_unsent_and_keeps_why_when_the_process_ended_after_an_answer() {
        let mut requests = answering_once(Ok(exit(1, "Error: the server stopped")), Then::EndsLater);
        assert_eq!(requests.ask(&line(&words(&["status"]), None)).await, answered(r#"["status"]"#));
        assert_eq!(requests.ask(&line(&words(&["focus"]), None)).await, Asked::Unsent);
        let stopped = Error::Failed { code: 1, stderr: "Error: the server stopped".to_owned() };
        assert_eq!(requests.ended().await, Some(stopped));
    }

    #[rstest]
    #[case::before_the_line_without_the_command(NO_REQUESTS_COMMAND, false)]
    #[case::before_the_line_without_follow(NO_FOLLOW_FLAG, false)]
    #[case::after_the_line_without_the_command(NO_REQUESTS_COMMAND, true)]
    #[case::after_the_line_without_follow(NO_FOLLOW_FLAG, true)]
    #[tokio::test]
    async fn finds_a_cli_that_does_not_know_the_command(#[case] stderr: &str, #[case] reads: bool) {
        let mut requests = ending(Ok(exit(2, stderr)), reads);
        assert_eq!(requests.ask(&line(&words(&["status"]), None)).await, Asked::Absent);
    }

    #[rstest]
    #[case::rejected(Error::Rejected { status: 503 })]
    #[case::timeout(Error::Timeout)]
    #[case::unreachable(Error::Connect("connection refused".to_owned()))]
    #[tokio::test]
    async fn leaves_the_line_unsent_when_the_bridge_cannot_be_reached(#[case] error: Error) {
        let mut requests = ending(Err(error), true);
        assert_eq!(requests.ask(&line(&words(&["status"]), None)).await, Asked::Unsent);
    }

    #[tokio::test]
    async fn leaves_the_line_unsent_when_the_process_does_not_start() {
        let mut requests = Requests::start(Arc::new(FakeTransport::default()));
        assert_eq!(requests.ask(&line(&words(&["status"]), None)).await, Asked::Unsent);
    }

    #[tokio::test]
    async fn leaves_the_line_unsent_when_the_process_stopped_reading() {
        let mut requests = follow(async |stdin, _| {
            drop(stdin);
            tokio::task::yield_now().await;
            Ok(exit(0, ""))
        });
        assert_eq!(requests.ask(&line(&words(&["status"]), None)).await, Asked::Unsent);
        assert_eq!(requests.ended().await, None);
    }
}

mod ended {
    use super::*;

    #[tokio::test]
    async fn waits_while_the_process_runs() {
        let (seen, _written) = mpsc::unbounded_channel();
        let mut requests = echoing(seen);
        assert_eq!(requests.ended().now_or_never(), None);
        assert_eq!(requests.ask(&line(&words(&["status"]), None)).await, answered(r#"["status"]"#));
    }

    #[rstest]
    #[case::while_idle(Then::EndsLater)]
    #[case::with_its_last_answer(Then::Ends)]
    #[tokio::test]
    async fn comes_once_the_process_is_over_and_nothing_is_written_after(#[case] then: Then) {
        let mut requests = answering_once(Ok(exit(1, "Error: the server stopped")), then);
        assert_eq!(requests.ask(&line(&words(&["status"]), None)).await, answered(r#"["status"]"#));
        let stopped = Error::Failed { code: 1, stderr: "Error: the server stopped".to_owned() };
        assert_eq!(requests.ended().await, Some(stopped));
        assert_eq!(requests.ask(&line(&words(&["focus"]), None)).await, Asked::Unsent);
        assert_eq!(requests.ended().now_or_never(), Some(None));
    }

    #[rstest]
    #[case::said_nothing(Ok(exit(0, "")), Some(Error::Failed { code: 0, stderr: ENDED.to_owned() }))]
    #[case::dropped(Err(Error::Dropped), Some(Error::Dropped))]
    #[case::unopened(Err(Error::Timeout), None)]
    #[tokio::test]
    async fn tells_why_the_process_ended_unless_it_never_opened(#[case] end: End, #[case] stopped: Option<Error>) {
        let mut requests = answering_once(end, Then::EndsLater);
        assert_eq!(requests.ask(&line(&words(&["status"]), None)).await, answered(r#"["status"]"#));
        assert_eq!(requests.ended().await, stopped);
    }
}
