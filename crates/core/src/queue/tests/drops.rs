use super::*;

const STOPPED: &str = "Error: the server stopped";

#[derive(Clone, Copy)]
enum Ending {
    WithItsAnswer,
    AfterItsAnswer,
    Midway,
}

fn ending_once(
    fake: FakeTransport,
    ending: Ending,
    log: &mpsc::UnboundedSender<String>,
    starts: &Starts,
) -> FakeTransport {
    let (log, starts) = (log.clone(), starts.clone());
    fake.script(&REQUESTS, move |stdin, out| {
        let first = starts.add() == 0;
        end_once(first.then_some(ending), log.clone(), stdin, out)
    })
}

async fn end_once(
    ending: Option<Ending>,
    log: mpsc::UnboundedSender<String>,
    mut stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> End {
    let Some(ending) = ending else {
        return answer_lines(log, stdin, out).await;
    };
    if let Some(Stdin::Text(line)) = stdin.next().await {
        match ending {
            Ending::WithItsAnswer => say(&log, &mut out, &line).await?,
            Ending::AfterItsAnswer => {
                say(&log, &mut out, &line).await?;
                tokio::task::yield_now().await;
            }
            Ending::Midway => log.send(format!("lost {}", said(&line))).expect("the test reads the log"),
        }
    }
    Ok(exit(1, STOPPED))
}

fn stopped() -> Error {
    Error::Failed { code: 1, stderr: STOPPED.to_owned() }
}

mod requests {
    use super::*;

    #[rstest]
    #[case::to_the_caller(true, vec![])]
    #[case::to_on_error(false, vec![Error::Unanswered])]
    #[tokio::test]
    async fn tells_an_answer_was_lost_without_sending_the_command_again(
        #[case] waits: bool,
        #[case] noticed: Vec<Error>,
    ) {
        let (log, mut said) = heard();
        let starts = Starts::default();
        let Started { queue, mut notices, .. } =
            start(ending_once(FakeTransport::default(), Ending::Midway, &log, &starts));
        let status = queue.ask(words(&["status"]), None);
        if waits {
            assert_eq!(status.await, Err(Error::Unanswered));
        } else {
            drop(status);
        }
        assert_eq!(queue.ask(words(&["focus"]), None).await, Ok(done("focus")));
        assert_eq!(said.taken(), ["lost status", "focus"]);
        assert_eq!((starts.count(), notices.taken()), (2, noticed));
    }

    #[rstest]
    #[case::after_its_answer(Ending::AfterItsAnswer)]
    #[case::with_its_answer(Ending::WithItsAnswer)]
    #[tokio::test(start_paused = true)]
    async fn tells_it_ended_and_starts_a_new_one_for_the_next_command(#[case] ending: Ending) {
        let (log, mut said) = heard();
        let starts = Starts::default();
        let Started { queue, mut notices, .. } = start(ending_once(FakeTransport::default(), ending, &log, &starts));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        settled().await;
        assert_eq!((starts.count(), notices.taken()), (1, vec![stopped()]));
        assert_eq!(queue.ask(words(&["focus"]), None).await, Ok(done("focus")));
        assert_eq!(said.taken(), ["status", "focus"]);
        assert_eq!((starts.count(), notices.taken()), (2, vec![]));
    }

    #[rstest]
    #[case::unreachable(Err(Error::Connect("connection refused".to_owned())), 3)]
    #[case::an_old_cli(Ok(exit(2, NO_REQUESTS_COMMAND)), 1)]
    #[tokio::test]
    async fn tries_the_process_again_unless_the_cli_lacks_it(#[case] end: End, #[case] tries: usize) {
        let starts = Starts::default();
        let fake = FakeTransport::default().answer(&["status"], &["once"], &[], 0);
        let Started { queue, mut notices, .. } = start(refusing(fake, end, &starts));
        for _ in 0..3 {
            assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("once")));
        }
        assert_eq!((starts.count(), notices.taken()), (tries, vec![]));
    }
}
