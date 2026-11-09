use std::future::Future;
use std::iter;

use futures_util::future::BoxFuture;
use futures_util::stream::BoxStream;
use futures_util::{FutureExt, SinkExt, StreamExt};
use rstest::rstest;
use serde_json::json;
use tokio::sync::mpsc;
use tokio::time::Instant;

use super::*;
use crate::transport::fake::FakeTransport;
use crate::transport::{Closed, Exit, Out, OutSink, Stdin};

const NO_FOLLOW_FLAG: &str = "error: unexpected argument '--follow' found\n\nUsage: cornercase input [OPTIONS]";
const NO_SERVER: &str = "warning: slow\nError: no cornercase server is running\n\n";
const REFUSED: &str = r#"{"error":"there is no pane 9","events":1,"pane":9}"#;

trait Script: Fn(BoxStream<'static, Stdin>, OutSink) -> BoxFuture<'static, End> + Send + Sync + 'static {}

impl<S> Script for S where S: Fn(BoxStream<'static, Stdin>, OutSink) -> BoxFuture<'static, End> + Send + Sync + 'static {}

struct Notices(mpsc::UnboundedReceiver<Error>);

impl Notices {
    fn taken(&mut self) -> Vec<Error> {
        iter::from_fn(|| self.0.try_recv().ok()).collect()
    }
}

fn exit(code: i32, stderr: &str) -> Exit {
    Exit { code, stderr: stderr.to_owned() }
}

fn key(code: &str) -> Value {
    json!({ "key": { "code": code } })
}

fn crashed(code: i32, why: &str) -> Error {
    Error::Failed { code, stderr: why.to_owned() }
}

fn typist<S, F>(script: S) -> (Typist, Notices)
where
    S: Fn(BoxStream<'static, Stdin>, OutSink) -> F + Send + Sync + 'static,
    F: Future<Output = End> + Send + 'static,
{
    let (notice, notices) = mpsc::unbounded_channel();
    let on_error: OnError = Arc::new(move |error| notice.send(error).expect("the test reads the notices"));
    let transport = FakeTransport::default().script(&FOLLOW, script);
    (Typist::start(Arc::new(transport), on_error), Notices(notices))
}

async fn say(out: &mut OutSink, line: &str) -> Result<(), Error> {
    out.send(Out::Stdout(line.to_owned())).await.map_err(|Closed| Error::OutputClosed)
}

fn answering(per: usize, answer: &'static str, written: mpsc::UnboundedSender<Value>) -> impl Script {
    move |stdin, out| answer_every(per, answer, written.clone(), stdin, out).boxed()
}

async fn answer_every(
    per: usize,
    answer: &str,
    written: mpsc::UnboundedSender<Value>,
    mut stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> End {
    let mut read = 0;
    while let Some(Stdin::Text(line)) = stdin.next().await {
        written.send(serde_json::from_str(&line).expect("a JSON line")).expect("the test reads the lines");
        read += 1;
        if read % per == 0 {
            say(&mut out, answer).await?;
        }
    }
    Ok(exit(0, ""))
}

fn silent(written: mpsc::UnboundedSender<Value>) -> impl Script {
    answering(usize::MAX, "{}", written)
}

fn ending(answers: usize, reads: usize, end: End) -> impl Script {
    move |stdin, out| answer_then_end(answers, reads, end.clone(), stdin, out).boxed()
}

async fn answer_then_end(
    answers: usize,
    reads: usize,
    end: End,
    mut stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> End {
    for read in 0..reads {
        stdin.next().await;
        if read < answers {
            say(&mut out, "{}").await?;
        }
    }
    end
}

async fn idle(typist: &mut Typist) {
    assert!(time::timeout(Duration::from_secs(1), typist.ended()).await.is_err());
}

async fn type_keys(typist: &mut Typist, keys: &[(u64, &str)]) {
    for &(pane, code) in keys {
        assert_eq!(typist.send(pane, &key(code)).await, Typed::Done);
    }
}

mod line {
    use super::*;

    #[test]
    fn names_the_pane_it_types_into() {
        let sent: Value = serde_json::from_str(&line(14, &key("enter"))).expect("a JSON line");
        assert_eq!(sent, json!({ "pane": 14, "key": { "code": "enter" } }));
    }

    #[test]
    fn stays_one_line_whatever_the_paste_holds() {
        let sent = line(3, &json!({ "paste": "two\nlines\r\n" }));
        assert!(!sent.contains('\n'));
        assert_eq!(serde_json::from_str::<Value>(&sent).expect("a JSON line")["paste"], "two\nlines\r\n");
    }

    #[test]
    fn leaves_an_event_that_is_not_an_object_as_it_is() {
        assert_eq!(line(3, &json!("enter")), r#""enter""#);
    }
}

mod answer {
    use super::*;

    #[rstest]
    #[case::its_events(r#"{"bytes":3,"events":3,"pane":45,"skipped":[]}"#, 3)]
    #[case::no_events("{}", 1)]
    #[case::events_that_are_not_a_number(r#"{"events":"3"}"#, 1)]
    fn says_how_many_lines_it_answers(#[case] line: &str, #[case] events: usize) {
        assert_eq!(answer(line), (events, None));
    }

    #[test]
    fn gives_the_error_of_a_refused_line() {
        assert_eq!(answer(REFUSED), (1, Some(Error::InputRejected("there is no pane 9".to_owned()))));
    }

    #[test]
    fn counts_a_line_that_is_not_json_as_one_failure_without_its_text() {
        let (events, error) = answer("oops secret");
        assert_eq!((events, error.clone()), (1, Some(Error::Json("not JSON at line 1 column 1".to_owned()))));
        assert!(!error.expect("an error").to_string().contains("secret"));
    }
}

mod send {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn writes_each_event_without_waiting_for_its_answer() {
        let (seen, mut written) = mpsc::unbounded_channel();
        let (mut typist, mut notices) = typist(silent(seen));
        type_keys(&mut typist, &[(14, "a"), (14, "enter"), (3, "b")]).await;
        idle(&mut typist).await;
        let lines: Vec<Value> = iter::from_fn(|| written.try_recv().ok()).collect();
        let keys = [(14, "a"), (14, "enter"), (3, "b")];
        let expected: Vec<Value> =
            keys.iter().map(|&(pane, code)| json!({ "pane": pane, "key": { "code": code } })).collect();
        assert_eq!((lines, typist.unanswered, notices.taken()), (expected, 3, vec![]));
    }

    #[tokio::test(start_paused = true)]
    async fn waits_for_the_answers_once_256_lines_are_unanswered() {
        let (seen, _written) = mpsc::unbounded_channel();
        let (mut typist, _notices) = typist(answering(UNANSWERED, r#"{"events":256}"#, seen));
        type_keys(&mut typist, &[(1, "a"); UNANSWERED]).await;
        assert_eq!(typist.unanswered, UNANSWERED);
        let started = Instant::now();
        type_keys(&mut typist, &[(1, "b")]).await;
        assert_eq!((typist.unanswered, started.elapsed()), (1, Duration::ZERO));
    }

    #[tokio::test(start_paused = true)]
    async fn gives_up_after_10_seconds_when_256_lines_stay_unanswered() {
        let (seen, _written) = mpsc::unbounded_channel();
        let (mut typist, mut notices) = typist(silent(seen));
        type_keys(&mut typist, &[(1, "a"); UNANSWERED]).await;
        let started = Instant::now();
        assert_eq!(typist.send(1, &key("b")).await, Typed::Gone);
        assert_eq!((started.elapsed(), notices.taken()), (SETTLE, vec![Error::SettleTimeout]));
    }

    #[rstest]
    #[case::unreachable(Err(Error::Connect("refused".to_owned())), Typed::Unsent(vec![]), vec![])]
    #[case::an_old_cli(Ok(exit(2, NO_FOLLOW_FLAG)), Typed::Absent(vec![]), vec![])]
    #[case::crashed(Ok(exit(1, NO_SERVER)), Typed::Gone, vec![crashed(1, "Error: no cornercase server is running")])]
    #[case::dropped(Err(Error::Dropped), Typed::Gone, vec![Error::Dropped])]
    #[tokio::test]
    async fn says_why_the_typist_did_not_start(#[case] end: End, #[case] typed: Typed, #[case] told: Vec<Error>) {
        let (mut typist, mut notices) = typist(ending(0, 0, end));
        assert_eq!(typist.send(14, &key("a")).await, typed);
        assert_eq!(notices.taken(), told);
    }

    #[tokio::test]
    async fn goes_on_after_a_refused_event() {
        let (seen, _written) = mpsc::unbounded_channel();
        let (mut typist, mut notices) = typist(answering(1, REFUSED, seen));
        type_keys(&mut typist, &[(9, "a"), (9, "b")]).await;
        assert_eq!(typist.settle().await, Typed::Done);
        let refused = Error::InputRejected("there is no pane 9".to_owned());
        assert_eq!(notices.taken(), vec![refused.clone(), refused]);
        type_keys(&mut typist, &[(9, "c")]).await;
    }

    #[tokio::test]
    async fn leaves_the_event_unsent_once_the_typist_ended() {
        let (mut typist, mut notices) = typist(ending(0, 1, Ok(exit(1, NO_SERVER))));
        type_keys(&mut typist, &[(14, "a")]).await;
        assert_eq!(typist.ended().await, Typed::Gone);
        assert_eq!(notices.taken().len(), 1);
        assert_eq!(typist.send(14, &key("b")).await, Typed::Gone);
        assert_eq!(notices.taken(), vec![]);
    }
}

mod settle {
    use super::*;

    #[rstest]
    #[case::one_answer_for_three_lines(3, 3, r#"{"bytes":3,"events":3,"pane":45,"skipped":[]}"#)]
    #[case::an_answer_per_line_without_events(2, 1, "{}")]
    #[case::an_answer_for_more_lines_than_written(1, 1, r#"{"events":5}"#)]
    #[tokio::test(start_paused = true)]
    async fn counts_the_lines_each_answer_covers(
        #[case] lines: usize,
        #[case] per: usize,
        #[case] answer: &'static str,
    ) {
        let (seen, _written) = mpsc::unbounded_channel();
        let (mut typist, mut notices) = typist(answering(per, answer, seen));
        type_keys(&mut typist, &vec![(45, "a"); lines]).await;
        let started = Instant::now();
        assert_eq!(typist.settle().await, Typed::Done);
        assert_eq!((typist.unanswered, started.elapsed(), notices.taken()), (0, Duration::ZERO, vec![]));
    }

    #[tokio::test]
    async fn is_done_at_once_with_every_line_answered() {
        let (seen, _written) = mpsc::unbounded_channel();
        let (mut typist, _notices) = typist(silent(seen));
        assert_eq!(typist.settle().now_or_never(), Some(Typed::Done));
    }

    #[tokio::test(start_paused = true)]
    async fn waits_for_an_answer_that_comes_in_time() {
        let (mut typist, mut notices) = typist(async |mut stdin, mut out| {
            stdin.next().await;
            tokio::time::sleep(Duration::from_secs(9)).await;
            say(&mut out, "{}").await?;
            stdin.next().await;
            Ok(exit(0, ""))
        });
        type_keys(&mut typist, &[(14, "a")]).await;
        let started = Instant::now();
        assert_eq!(typist.settle().await, Typed::Done);
        assert_eq!((started.elapsed(), notices.taken()), (Duration::from_secs(9), vec![]));
    }

    #[rstest]
    #[case::without_answers(usize::MAX, "{}")]
    #[case::with_answers_that_cover_fewer_lines(2, r#"{"events":1}"#)]
    #[tokio::test(start_paused = true)]
    async fn gives_up_after_10_seconds(#[case] per: usize, #[case] answer: &'static str) {
        let (seen, _written) = mpsc::unbounded_channel();
        let (mut typist, mut notices) = typist(answering(per, answer, seen));
        type_keys(&mut typist, &[(14, "a"), (14, "b")]).await;
        let started = Instant::now();
        assert_eq!(typist.settle().await, Typed::Gone);
        assert_eq!((started.elapsed(), notices.taken()), (SETTLE, vec![Error::SettleTimeout]));
    }

    #[rstest]
    #[case::an_old_cli(Ok(exit(2, NO_FOLLOW_FLAG)), Typed::Absent(vec![(14, key("a")), (3, key("b"))]))]
    #[case::unreachable(Err(Error::Timeout), Typed::Unsent(vec![(14, key("a")), (3, key("b"))]))]
    #[tokio::test]
    async fn hands_back_the_events_that_never_ran(#[case] end: End, #[case] typed: Typed) {
        let (mut typist, mut notices) = typist(ending(0, 2, end));
        type_keys(&mut typist, &[(14, "a"), (3, "b")]).await;
        assert_eq!((typist.settle().await, notices.taken()), (typed, vec![]));
    }

    #[rstest]
    #[case::crashed(Ok(exit(1, NO_SERVER)), crashed(1, "Error: no cornercase server is running"))]
    #[case::an_old_cli_message_after_an_answer(
        Ok(exit(2, NO_FOLLOW_FLAG)),
        crashed(2, "Usage: cornercase input [OPTIONS]")
    )]
    #[case::a_clean_exit(Ok(exit(0, "")), crashed(0, "`cornercase input --follow` ended"))]
    #[case::dropped(Err(Error::Dropped), Error::Dropped)]
    #[tokio::test]
    async fn tells_why_the_typist_ended_with_lines_unanswered(#[case] end: End, #[case] why: Error) {
        let (mut typist, mut notices) = typist(ending(1, 2, end));
        type_keys(&mut typist, &[(14, "a"), (14, "b")]).await;
        assert_eq!((typist.settle().await, notices.taken()), (Typed::Gone, vec![why]));
    }
}

mod ended {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn tells_a_refusal_while_the_typist_runs() {
        let (seen, _written) = mpsc::unbounded_channel();
        let (mut typist, mut notices) = typist(answering(1, REFUSED, seen));
        type_keys(&mut typist, &[(9, "a")]).await;
        idle(&mut typist).await;
        assert_eq!(notices.taken(), vec![Error::InputRejected("there is no pane 9".to_owned())]);
    }

    #[rstest]
    #[case::an_old_cli(Ok(exit(2, NO_FOLLOW_FLAG)), Typed::Absent(vec![]), vec![])]
    #[case::unreachable(Err(Error::Unauthorized), Typed::Unsent(vec![]), vec![])]
    #[case::crashed(Ok(exit(1, NO_SERVER)), Typed::Gone, vec![crashed(1, "Error: no cornercase server is running")])]
    #[tokio::test]
    async fn says_how_the_typist_ended_while_idle(#[case] end: End, #[case] typed: Typed, #[case] told: Vec<Error>) {
        let (mut typist, mut notices) = typist(ending(0, 0, end));
        assert_eq!((typist.ended().await, notices.taken()), (typed, told));
    }

    #[tokio::test]
    async fn tells_a_refusal_that_came_with_the_end_of_the_typist() {
        let script = |mut stdin: BoxStream<'static, Stdin>, mut out: OutSink| {
            async move {
                stdin.next().await;
                say(&mut out, REFUSED).await?;
                Ok(exit(1, NO_SERVER))
            }
            .boxed()
        };
        let (mut typist, mut notices) = typist(script);
        type_keys(&mut typist, &[(9, "a")]).await;
        assert_eq!(typist.ended().await, Typed::Gone);
        let told = notices.taken();
        assert_eq!(told.first(), Some(&Error::InputRejected("there is no pane 9".to_owned())));
    }

    #[tokio::test]
    async fn comes_at_once_and_says_nothing_more_once_the_typist_is_over() {
        let (mut typist, mut notices) = typist(ending(0, 0, Ok(exit(1, NO_SERVER))));
        assert_eq!(typist.ended().await, Typed::Gone);
        assert_eq!(notices.taken().len(), 1);
        assert_eq!((typist.ended().now_or_never(), notices.taken()), (Some(Typed::Gone), vec![]));
    }
}
