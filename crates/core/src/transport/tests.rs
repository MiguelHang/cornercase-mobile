use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use futures_util::{SinkExt, StreamExt, sink, stream};
use rstest::rstest;

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
}
