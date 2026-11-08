use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use futures_util::{SinkExt, StreamExt, sink, stream};
use rstest::rstest;
use serde::Deserialize;

use super::*;

mod transport {
    use super::*;

    struct Counting;

    impl Transport for Counting {
        fn run(
            &self,
            argv: Vec<String>,
            stdin: BoxStream<'static, Stdin>,
            mut out: OutSink,
        ) -> BoxFuture<'_, Result<Exit, Error>> {
            Box::pin(async move {
                let inputs = stdin.count().await;
                out.send(Out::Stdout(argv.join(" "))).await.map_err(|Closed| Error::OutputClosed)?;
                Ok(Exit { code: i32::try_from(inputs).expect("a few inputs"), stderr: String::new() })
            })
        }
    }

    fn shareable<T: Send + Sync + ?Sized>() {}

    fn run_now(transport: &dyn Transport) -> Result<Exit, Error> {
        let stdin = stream::iter([Stdin::Text("a".to_owned()), Stdin::Bytes(vec![0, 1])]).boxed();
        let out: OutSink = Box::pin(sink::drain().sink_map_err(|never| match never {}));
        let mut running = transport.run(vec!["status".to_owned(), "--json".to_owned()], stdin, out);
        match running.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(result) => result,
            Poll::Pending => panic!("the counting transport never waits"),
        }
    }

    #[test]
    fn is_shared_between_threads() {
        shareable::<dyn Transport>();
        shareable::<Arc<dyn Transport>>();
    }

    #[test]
    fn runs_behind_a_box() {
        let transport: Box<dyn Transport> = Box::new(Counting);
        assert_eq!(run_now(transport.as_ref()), Ok(Exit { code: 2, stderr: String::new() }));
    }

    #[test]
    fn runs_behind_an_arc() {
        let transport: Arc<dyn Transport> = Arc::new(Counting);
        assert_eq!(run_now(transport.as_ref()), Ok(Exit { code: 2, stderr: String::new() }));
    }

    #[test]
    fn reports_a_closed_output() {
        let stdin = stream::empty().boxed();
        let out: OutSink = Box::pin(sink::unfold((), |(), _: Out| async { Err::<(), _>(Closed) }));
        let mut running = Counting.run(Vec::new(), stdin, out);
        let polled = running.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        assert_eq!(polled, Poll::Ready(Err(Error::OutputClosed)));
    }
}

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|&word| word.to_owned()).collect()
}

mod output {
    use super::*;
    use crate::transport::fake::FakeTransport;

    #[rstest]
    #[case::nothing(&[], "")]
    #[case::one_line(&["main"], "main")]
    #[case::several_lines(&["a", "b", "c"], "a\nb\nc")]
    #[case::empty_lines(&["", "a", ""], "\na\n")]
    #[tokio::test]
    async fn joins_the_stdout_lines_with_a_newline(#[case] lines: &[&str], #[case] stdout: &str) {
        let fake = FakeTransport::default().answer(&["list"], lines, &[], 0);
        let output = output(&fake, words(&["list"])).await.expect("an answer");
        assert_eq!(output.stdout, stdout);
    }

    #[tokio::test]
    async fn keeps_the_stderr_and_the_code_apart() {
        let fake = FakeTransport::default().answer(&["kill"], &["half"], &["warning: a", "error: b"], 3);
        let output = output(&fake, words(&["kill"])).await;
        let expected = Output { stdout: "half".to_owned(), stderr: "warning: a\nerror: b".to_owned(), code: 3 };
        assert_eq!(output, Ok(expected));
    }

    #[tokio::test]
    async fn sends_no_input_and_ends_it_at_once() {
        let fake = FakeTransport::default().script(&["read"], async |stdin, mut out| {
            let inputs = stdin.count().await;
            out.send(Out::Stdout(format!("{inputs} inputs"))).await.map_err(|Closed| Error::OutputClosed)?;
            Ok(Exit { code: 0, stderr: String::new() })
        });
        let output = output(&fake, words(&["read"])).await.expect("an answer");
        assert_eq!(output.stdout, "0 inputs");
    }

    #[tokio::test]
    async fn passes_the_transport_error_on() {
        let fake = FakeTransport::default().fail(&["status"], Error::Dropped);
        assert_eq!(output(&fake, words(&["status"])).await, Err(Error::Dropped));
    }
}

mod listen {
    use super::*;

    #[tokio::test]
    async fn hands_on_only_what_it_keeps() {
        let (mut out, mut receiver) = listen(only_stdout);
        let lines = [Out::Stderr("a".to_owned()), Out::Stdout("b".to_owned()), Out::Stderr("c".to_owned())];
        out.send_all(&mut stream::iter(lines.map(Ok))).await.expect("someone listens");
        drop(out);
        assert_eq!(iter::from_fn(|| receiver.try_recv().ok()).collect::<Vec<_>>(), ["b"]);
    }
}

mod json {
    use super::*;
    use crate::transport::fake::FakeTransport;

    #[derive(Debug, PartialEq, Eq, Deserialize)]
    struct Contract {
        version: u32,
        requests: Vec<String>,
    }

    async fn contract(fake: &FakeTransport) -> Result<Contract, Error> {
        json(fake, words(&["contract", "--json"])).await
    }

    #[tokio::test]
    async fn reads_a_json_spread_over_lines() {
        let lines = ["{", "  \"version\": 2,", "  \"requests\": [\"status\", \"watch\"]", "}"];
        let fake = FakeTransport::default().answer(&["contract", "--json"], &lines, &[], 0);
        let expected = Contract { version: 2, requests: words(&["status", "watch"]) };
        assert_eq!(contract(&fake).await, Ok(expected));
    }

    #[tokio::test]
    async fn adds_nothing_to_the_argv() {
        let fake = FakeTransport::default().answer(&["contract"], &["{}"], &[], 0);
        let parsed: Result<serde_json::Value, Error> = json(&fake, words(&["contract"])).await;
        assert_eq!(parsed, Ok(serde_json::json!({})));
    }

    #[tokio::test]
    async fn fails_with_the_code_and_the_stderr() {
        let fake = FakeTransport::default().answer(&["contract", "--json"], &["{}"], &["error: no server"], 2);
        assert_eq!(contract(&fake).await, Err(Error::Failed { code: 2, stderr: "error: no server".to_owned() }));
    }

    #[tokio::test]
    async fn passes_the_transport_error_on() {
        let fake = FakeTransport::default().fail(&["contract", "--json"], Error::Unauthorized);
        assert_eq!(contract(&fake).await, Err(Error::Unauthorized));
    }

    #[rstest]
    #[case::syntax(&["secret-token"], "not JSON at line 1 column 1")]
    #[case::wrong_type(&["{\"version\": \"secret-token\"}"], "not the expected shape at line 1 column 26")]
    #[case::too_short(&["{\"version\": 2,", "\"requests\": [\"secret\""], "the JSON ends too early at line 2 column 21")]
    #[tokio::test]
    async fn names_the_place_but_not_the_text(#[case] lines: &[&str], #[case] reason: &str) {
        let fake = FakeTransport::default().answer(&["contract", "--json"], lines, &[], 0);
        let error = contract(&fake).await.expect_err("not a contract");
        assert_eq!(error, Error::Json(reason.to_owned()));
        assert!(!error.to_string().contains("secret"), "the message leaks the output: {error}");
    }
}

mod stderr_tail {
    use super::*;

    fn collect(lines: &[&str]) -> String {
        let mut tail = StderrTail::default();
        for line in lines {
            tail.push(line);
        }
        tail.finish()
    }

    #[rstest]
    #[case::nothing(&[], "")]
    #[case::one_line(&["error: no such project"], "error: no such project")]
    #[case::several_lines(&["warning: a", "error: b"], "warning: a\nerror: b")]
    #[case::empty_lines(&["", "a", ""], "\na\n")]
    fn joins_the_lines_with_a_newline(#[case] lines: &[&str], #[case] joined: &str) {
        assert_eq!(collect(lines), joined);
    }

    #[rstest]
    #[case::one_long_line(vec!["x".repeat(3 * StderrTail::LIMIT) + "end"])]
    #[case::many_lines(vec!["y".repeat(1000); 300].into_iter().chain(["end".to_owned()]).collect())]
    fn keeps_the_last_64_kib(#[case] lines: Vec<String>) {
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        let tail = collect(&lines);
        assert_eq!(tail.len(), StderrTail::LIMIT);
        assert!(tail.ends_with("end"), "the tail ends with the last line");
    }

    #[rstest]
    #[case::two_bytes("ü")]
    #[case::three_bytes("€")]
    #[case::four_bytes("🦀")]
    fn never_cuts_a_character(#[case] character: &str) {
        let line = format!("a{}", character.repeat(StderrTail::LIMIT));
        let tail = collect(&[line.as_str()]);
        assert!(tail.len() <= StderrTail::LIMIT && tail.len() > StderrTail::LIMIT - character.len());
        assert_eq!(tail.trim_start_matches(character), "", "only whole characters are kept");
    }

    #[rstest]
    #[case::just_over_the_limit(StderrTail::LIMIT + 1, StderrTail::LIMIT + 1)]
    #[case::one_and_a_half_limits(3 * StderrTail::LIMIT / 2, 3 * StderrTail::LIMIT / 2)]
    #[case::twice_the_limit(2 * StderrTail::LIMIT, 2 * StderrTail::LIMIT)]
    #[case::past_twice_the_limit(2 * StderrTail::LIMIT + 1, StderrTail::LIMIT)]
    fn trims_what_it_holds_only_past_twice_the_limit(#[case] length: usize, #[case] held: usize) {
        let mut tail = StderrTail::default();
        tail.push(&"x".repeat(length));
        assert_eq!(tail.text.len(), held);
    }

    #[test]
    fn holds_no_more_than_twice_the_limit_between_lines() {
        let mut tail = StderrTail::default();
        for _ in 0..1000 {
            tail.push(&"y".repeat(1000));
            assert!(tail.text.len() <= 2 * StderrTail::LIMIT + 1000);
        }
    }
}
