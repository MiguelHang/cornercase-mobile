use tokio::time::Instant;

use super::*;

const TYPIST: [&str; 4] = ["input", "--follow", "--json", "-"];
pub(super) const NO_FOLLOW_FLAG: &str =
    "error: unexpected argument '--follow' found\n\nUsage: cornercase input [OPTIONS]";
const REFUSED: &str = r#"{"error":"there is no pane 9","events":1,"pane":9}"#;
const STOPPED: &str = "Error: the server stopped";
const SETTLE: Duration = Duration::from_secs(10);

#[derive(Clone, Copy)]
pub(super) struct Answers {
    per: usize,
    answer: &'static str,
    after: Duration,
}

pub(super) const EACH: Answers = Answers { per: 1, answer: "{}", after: Duration::ZERO };
pub(super) const SILENT: Answers = Answers { per: usize::MAX, ..EACH };

pub(super) fn key(code: &str) -> Value {
    json!({ "key": code })
}

fn refused() -> Error {
    Error::InputRejected("there is no pane 9".to_owned())
}

pub(super) fn typing(
    fake: FakeTransport,
    log: &mpsc::UnboundedSender<String>,
    starts: &Starts,
    answers: Answers,
) -> FakeTransport {
    let (log, starts) = (log.clone(), starts.clone());
    fake.script(&TYPIST, move |stdin, out| {
        starts.add();
        type_keys(log.clone(), answers, stdin, out)
    })
}

fn started(log: &mpsc::UnboundedSender<String>, starts: &Starts, answers: Answers) -> Started {
    start(typing(answering(FakeTransport::default(), log, &Starts::default()), log, starts, answers))
}

fn crashing(fake: FakeTransport, log: &mpsc::UnboundedSender<String>, starts: &Starts, reads: bool) -> FakeTransport {
    let (log, starts) = (log.clone(), starts.clone());
    fake.script(&TYPIST, move |stdin, out| {
        let first = starts.add() == 0;
        type_then_end(first, reads, log.clone(), stdin, out)
    })
}

pub(super) fn unstarted(fake: FakeTransport, end: End, reads: bool, starts: &Starts) -> FakeTransport {
    let starts = starts.clone();
    fake.script(&TYPIST, move |stdin, _| {
        starts.add();
        read_then_end(reads, stdin, end.clone())
    })
}

pub(super) fn typing_once(fake: FakeTransport, log: &mpsc::UnboundedSender<String>, panes: &[u64]) -> FakeTransport {
    panes.iter().fold(fake, |fake, &pane| {
        let log = log.clone();
        let argv = ["input", "--pane", &pane.to_string(), "--json", "-"];
        fake.script(&argv, move |stdin, _| type_once(pane, log.clone(), stdin))
    })
}

fn typed(line: &str) -> String {
    let line: Value = serde_json::from_str(line).expect("an input line");
    format!("typed {} {}", line["pane"], line["key"].as_str().expect("its key"))
}

async fn reply(out: &mut OutSink, line: &str) -> Result<(), Error> {
    out.send(Out::Stdout(line.to_owned())).await.map_err(|Closed| Error::OutputClosed)
}

async fn type_keys(
    log: mpsc::UnboundedSender<String>,
    answers: Answers,
    mut stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> End {
    let mut read = 0;
    while let Some(Stdin::Text(line)) = stdin.next().await {
        log.send(typed(&line)).expect("the test reads the log");
        read += 1;
        if read % answers.per == 0 {
            time::sleep(answers.after).await;
            reply(&mut out, answers.answer).await?;
        }
    }
    Ok(exit(0, ""))
}

async fn type_then_end(
    first: bool,
    reads: bool,
    log: mpsc::UnboundedSender<String>,
    mut stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> End {
    if !first {
        return type_keys(log, EACH, stdin, out).await;
    }
    if let Some(Stdin::Text(line)) = stdin.next().await {
        log.send(typed(&line)).expect("the test reads the log");
        reply(&mut out, "{}").await?;
    }
    if reads && let Some(Stdin::Text(line)) = stdin.next().await {
        log.send(typed(&line)).expect("the test reads the log");
    }
    tokio::task::yield_now().await;
    Ok(exit(1, STOPPED))
}

async fn read_then_end(reads: bool, mut stdin: BoxStream<'static, Stdin>, end: End) -> End {
    if reads {
        stdin.next().await;
    }
    end
}

async fn type_once(pane: u64, log: mpsc::UnboundedSender<String>, stdin: BoxStream<'static, Stdin>) -> End {
    let read: Vec<String> = stdin.filter_map(async |stdin| text(stdin)).collect().await;
    let lines = read.concat();
    log.send(format!("once {pane} < {}", lines.trim_end().replace('\n', " "))).expect("the test reads the log");
    Ok(exit(0, ""))
}

mod keys {
    use super::*;

    #[tokio::test]
    async fn types_keys_and_commands_in_the_order_they_were_queued() {
        let (log, mut said) = heard();
        let starts = Starts::default();
        let Started { queue, mut notices, .. } = started(&log, &starts, EACH);
        queue.input(5, key("a"));
        queue.run(words(&["focus", "--pane", "7"]), None);
        queue.input(7, key("b"));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!(said.taken(), ["typed 5 a", "focus --pane 7", "typed 7 b", "status"]);
        assert_eq!((starts.count(), notices.taken()), (1, vec![]));
    }

    #[tokio::test(start_paused = true)]
    async fn writes_the_keys_without_waiting_for_their_answers() {
        let (log, mut said) = heard();
        let three = Answers { per: 3, answer: r#"{"events":3}"#, after: Duration::ZERO };
        let Started { queue, mut notices, .. } = started(&log, &Starts::default(), three);
        for code in ["a", "b", "c"] {
            queue.input(5, key(code));
        }
        let started = Instant::now();
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!(said.taken(), ["typed 5 a", "typed 5 b", "typed 5 c", "status"]);
        assert_eq!((started.elapsed(), notices.taken()), (Duration::ZERO, vec![]));
    }

    #[tokio::test]
    async fn goes_on_after_a_refused_key() {
        let (log, _said) = heard();
        let refusing = Answers { answer: REFUSED, ..EACH };
        let Started { queue, mut notices, .. } = started(&log, &Starts::default(), refusing);
        queue.input(9, key("a"));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!(notices.taken(), [refused()]);
    }

    #[rstest]
    #[case::an_old_cli(Ok(exit(2, NO_FOLLOW_FLAG)), false, 1)]
    #[case::an_old_cli_after_a_line(Ok(exit(2, NO_FOLLOW_FLAG)), true, 1)]
    #[case::unreachable(Err(Error::Connect("connection refused".to_owned())), false, 2)]
    #[case::unreachable_after_a_line(Err(Error::Timeout), true, 2)]
    #[tokio::test]
    async fn types_once_per_run_of_one_pane_when_the_typist_does_not_start(
        #[case] end: End,
        #[case] reads: bool,
        #[case] tries: usize,
    ) {
        let (log, mut said) = heard();
        let starts = Starts::default();
        let fake = answering(FakeTransport::default(), &log, &Starts::default());
        let Started { queue, mut notices, .. } =
            start(unstarted(typing_once(fake, &log, &[5, 7]), end, reads, &starts));
        for (pane, code) in [(5, "a"), (5, "b"), (7, "c"), (5, "d")] {
            queue.input(pane, key(code));
        }
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        queue.input(5, key("e"));
        assert_eq!(queue.ask(words(&["focus"]), None).await, Ok(done("focus")));
        let once = [r#"once 5 < {"key":"a"} {"key":"b"}"#, r#"once 7 < {"key":"c"}"#, r#"once 5 < {"key":"d"}"#];
        assert_eq!(said.taken(), [&once[..], &["status", r#"once 5 < {"key":"e"}"#, "focus"]].concat());
        assert_eq!((starts.count(), notices.taken()), (tries, vec![]));
    }

    #[rstest]
    #[case::some_not_sent(3)]
    #[case::all_sent(0)]
    #[tokio::test]
    async fn tells_a_key_typed_once_was_refused(#[case] code: i32) {
        let (log, _said) = heard();
        let fake = answering(FakeTransport::default(), &log, &Starts::default()).answer(
            &["input", "--pane", "9", "--json", "-"],
            &[REFUSED],
            &[],
            code,
        );
        let Started { queue, mut notices, .. } =
            start(unstarted(fake, Ok(exit(2, NO_FOLLOW_FLAG)), false, &Starts::default()));
        queue.input(9, key("a"));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!(notices.taken(), [refused()]);
    }
}

mod settle {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn waits_for_the_keys_to_be_answered_before_a_command() {
        let (log, mut said) = heard();
        let late = Answers { after: Duration::from_secs(9), ..EACH };
        let Started { queue, mut notices, .. } = started(&log, &Starts::default(), late);
        queue.input(5, key("a"));
        let started = Instant::now();
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!(said.taken(), ["typed 5 a", "status"]);
        assert_eq!((started.elapsed(), notices.taken()), (Duration::from_secs(9), vec![]));
    }

    #[tokio::test(start_paused = true)]
    async fn gives_up_after_10_seconds_and_types_the_next_key_through_a_new_typist() {
        let (log, mut said) = heard();
        let starts = Starts::default();
        let Started { queue, mut notices, .. } = started(&log, &starts, SILENT);
        queue.input(5, key("a"));
        let started = Instant::now();
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!((started.elapsed(), notices.taken()), (SETTLE, vec![Error::SettleTimeout]));
        queue.input(5, key("b"));
        settled().await;
        assert_eq!(said.taken(), ["typed 5 a", "status", "typed 5 b"]);
        assert_eq!(starts.count(), 2);
    }
}

mod typist {
    use super::*;

    fn stopped() -> Error {
        Error::Failed { code: 1, stderr: STOPPED.to_owned() }
    }

    #[tokio::test]
    async fn tells_it_ended_with_a_key_unanswered_and_does_not_type_that_key_again() {
        let (log, mut said) = heard();
        let starts = Starts::default();
        let fake = crashing(answering(FakeTransport::default(), &log, &Starts::default()), &log, &starts, true);
        let Started { queue, mut notices, .. } = start(fake);
        queue.input(5, key("a"));
        queue.input(5, key("b"));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!(notices.taken(), [stopped()]);
        queue.input(5, key("c"));
        assert_eq!(queue.ask(words(&["focus"]), None).await, Ok(done("focus")));
        assert_eq!(said.taken(), ["typed 5 a", "typed 5 b", "status", "typed 5 c", "focus"]);
        assert_eq!((starts.count(), notices.taken()), (2, vec![]));
    }

    #[tokio::test(start_paused = true)]
    async fn tells_it_ended_while_idle_and_types_the_next_key_through_a_new_one() {
        let (log, mut said) = heard();
        let starts = Starts::default();
        let fake = crashing(answering(FakeTransport::default(), &log, &Starts::default()), &log, &starts, false);
        let Started { queue, mut notices, .. } = start(fake);
        queue.input(5, key("a"));
        settled().await;
        assert_eq!(notices.taken(), [stopped()]);
        queue.input(5, key("b"));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!(said.taken(), ["typed 5 a", "typed 5 b", "status"]);
        assert_eq!((starts.count(), notices.taken()), (2, vec![]));
    }

    #[tokio::test(start_paused = true)]
    async fn types_once_the_keys_it_never_got_through_when_it_ends_while_idle() {
        let (log, mut said) = heard();
        let fake = typing_once(FakeTransport::default(), &log, &[5]);
        let Started { queue, mut notices, .. } = start(unstarted(fake, Err(Error::Timeout), true, &Starts::default()));
        queue.input(5, key("a"));
        settled().await;
        assert_eq!((said.taken(), notices.taken()), (vec![r#"once 5 < {"key":"a"}"#.to_owned()], vec![]));
    }

    #[tokio::test(start_paused = true)]
    async fn has_its_keys_answered_before_the_worker_ends() {
        let (log, mut said) = heard();
        let late = Answers { answer: REFUSED, after: Duration::from_secs(1), ..EACH };
        let Started { queue, mut notices, worker } =
            start(typing(FakeTransport::default(), &log, &Starts::default(), late));
        queue.input(9, key("a"));
        drop(queue);
        worker.await.expect("the worker ends");
        assert_eq!((said.taken(), notices.taken()), (vec!["typed 9 a".to_owned()], vec![refused()]));
    }
}
