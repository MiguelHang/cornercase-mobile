use super::*;

fn shareable<T: Send + Sync>(_: &T) {}

fn ending_once_idle(fake: FakeTransport, log: &mpsc::UnboundedSender<String>, starts: &Starts) -> FakeTransport {
    let (log, starts) = (log.clone(), starts.clone());
    fake.script(&REQUESTS, move |stdin, out| {
        let first = starts.add() == 0;
        answer_then_end(first, log.clone(), stdin, out)
    })
}

async fn answer_then_end(
    first: bool,
    log: mpsc::UnboundedSender<String>,
    mut stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> End {
    if !first {
        return answer_lines(log, stdin, out).await;
    }
    if let Some(Stdin::Text(line)) = stdin.next().await {
        say(&log, &mut out, &line).await?;
    }
    tokio::task::yield_now().await;
    Ok(exit(1, "Error: the server stopped"))
}

mod queue {
    use super::*;

    #[tokio::test]
    async fn is_shared_between_threads() {
        let Started { queue, .. } = start(FakeTransport::default());
        shareable(&queue);
    }
}

mod ask {
    use super::*;

    #[tokio::test]
    async fn runs_the_commands_in_order_through_one_process() {
        let (log, mut said) = heard();
        let starts = Starts::default();
        let Started { queue, .. } = start(answering(FakeTransport::default(), &log, &starts));
        let focus = queue.ask(words(&["focus", "--tab", "6"]), None);
        let send = queue.ask(words(&["send", "--pane", "5", "-"]), Some(Stdin::Text("ls".to_owned())));
        let status = queue.ask(words(&["status"]), None);
        assert_eq!(status.await, Ok(done("status")));
        assert_eq!(send.await, Ok(done("send --pane 5 - < ls")));
        assert_eq!(focus.await, Ok(done("focus --tab 6")));
        assert_eq!(said.taken(), ["focus --tab 6", "send --pane 5 - < ls", "status"]);
        assert_eq!(starts.count(), 1);
    }

    #[tokio::test]
    async fn goes_on_after_a_command_that_failed() {
        let (log, _said) = heard();
        let Started { queue, mut notices, .. } = start(answering(FakeTransport::default(), &log, &Starts::default()));
        assert_eq!(queue.ask(words(&["fail"]), None).await, Err(it_failed()));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!(notices.taken(), []);
    }

    #[rstest]
    #[case::not_scripted(None)]
    #[case::unreachable(Some(Err(Error::Connect("connection refused".to_owned()))))]
    #[case::unauthorized(Some(Err(Error::Unauthorized)))]
    #[case::an_old_cli(Some(Ok(exit(2, NO_REQUESTS_COMMAND))))]
    #[tokio::test]
    async fn runs_the_command_once_when_the_process_does_not_start(#[case] end: Option<End>) {
        let fake = reading(FakeTransport::default(), &["send", "--pane", "5", "-"]);
        let fake = match end {
            Some(end) => refusing(fake, end, &Starts::default()),
            None => fake,
        };
        let Started { queue, .. } = start(fake);
        let sent = queue.ask(words(&["send", "--pane", "5", "-"]), Some(Stdin::Text("ls".to_owned()))).await;
        assert_eq!(sent, Ok(done("once read ls")));
    }

    #[rstest]
    #[case::unreachable(Err(Error::Connect("connection refused".to_owned())), 3)]
    #[case::an_old_cli(Ok(exit(2, NO_REQUESTS_COMMAND)), 1)]
    #[tokio::test]
    async fn tries_the_process_again_unless_the_cli_lacks_it(#[case] end: End, #[case] tries: usize) {
        let starts = Starts::default();
        let fake = FakeTransport::default().answer(&["status"], &["once"], &[], 0);
        let Started { queue, .. } = start(refusing(fake, end, &starts));
        for _ in 0..3 {
            assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("once")));
        }
        assert_eq!(starts.count(), tries);
    }

    #[tokio::test(start_paused = true)]
    async fn starts_a_new_process_after_one_ended_while_idle() {
        let (log, mut said) = heard();
        let starts = Starts::default();
        let Started { queue, mut notices, .. } = start(ending_once_idle(FakeTransport::default(), &log, &starts));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        settled().await;
        assert_eq!(starts.count(), 1);
        assert_eq!(queue.ask(words(&["focus"]), None).await, Ok(done("focus")));
        assert_eq!(said.taken(), ["status", "focus"]);
        assert_eq!(starts.count(), 2);
        assert_eq!(notices.taken(), []);
    }

    #[tokio::test]
    async fn says_the_answer_was_lost_when_the_worker_is_gone() {
        let (queue, worker) = Queue::new(Arc::new(FakeTransport::default()), |_| {});
        drop(worker);
        assert_eq!(queue.ask(words(&["status"]), None).await, Err(Error::Unanswered));
    }
}

mod run {
    use super::*;

    #[tokio::test]
    async fn hands_a_failure_nobody_waits_for_to_on_error() {
        let (log, mut said) = heard();
        let Started { queue, mut notices, .. } = start(answering(FakeTransport::default(), &log, &Starts::default()));
        queue.run(words(&["fail"]), None);
        queue.run(words(&["status"]), None);
        assert_eq!(queue.ask(words(&["focus"]), None).await, Ok(done("focus")));
        assert_eq!(said.taken(), ["fail", "status", "focus"]);
        assert_eq!(notices.taken(), [it_failed()]);
    }

    #[rstest]
    #[case::a_failure("fail", vec![it_failed()])]
    #[case::an_answer("status", vec![])]
    #[tokio::test]
    async fn hands_on_error_what_a_dropped_ask_did_not_hear(#[case] command: &str, #[case] noticed: Vec<Error>) {
        let (log, _said) = heard();
        let Started { queue, mut notices, .. } = start(answering(FakeTransport::default(), &log, &Starts::default()));
        drop(queue.ask(words(&[command]), None));
        assert_eq!(queue.ask(words(&["focus"]), None).await, Ok(done("focus")));
        assert_eq!(notices.taken(), noticed);
    }
}

mod worker {
    use super::*;

    #[tokio::test]
    async fn ends_after_running_what_was_queued_once_the_last_handle_is_dropped() {
        let (log, mut said) = heard();
        let Started { queue, mut notices, worker } =
            start(answering(FakeTransport::default(), &log, &Starts::default()));
        let copy = queue.clone();
        queue.run(words(&["focus"]), None);
        copy.run(words(&["fail"]), None);
        drop(queue);
        copy.run(words(&["status"]), None);
        drop(copy);
        worker.await.expect("the worker ends");
        assert_eq!(said.taken(), ["focus", "fail", "status"]);
        assert_eq!(notices.taken(), [it_failed()]);
    }

    #[tokio::test(start_paused = true)]
    async fn waits_while_a_handle_is_left() {
        let (log, _said) = heard();
        let Started { queue, mut worker, .. } = start(answering(FakeTransport::default(), &log, &Starts::default()));
        let copy = queue.clone();
        drop(queue);
        assert!(time::timeout(Duration::from_secs(1), &mut worker).await.is_err());
        assert_eq!(copy.ask(words(&["status"]), None).await, Ok(done("status")));
    }
}
