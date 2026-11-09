use futures_util::FutureExt;
use futures_util::future::BoxFuture;

use super::input::{EACH, NO_FOLLOW_FLAG, SILENT, key, typing, typing_once, unstarted};
use super::*;

const CANCELLED: [&str; 2] = ["cancelled requests --follow -", "cancelled input --follow --json -"];

struct Watched {
    fake: FakeTransport,
    log: mpsc::UnboundedSender<String>,
}

struct Running(Option<(mpsc::UnboundedSender<String>, String)>);

impl Running {
    fn finish(&mut self) {
        self.0 = None;
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some((log, argv)) = self.0.take() {
            let _ = log.send(format!("cancelled {argv}"));
        }
    }
}

impl Transport for Watched {
    fn run(&self, argv: Vec<String>, stdin: BoxStream<'static, Stdin>, out: OutSink) -> BoxFuture<'_, End> {
        let mut running = Running(Some((self.log.clone(), argv.join(" "))));
        let run = self.fake.run(argv, stdin, out);
        async move {
            let end = run.await;
            running.finish();
            end
        }
        .boxed()
    }
}

fn watched(fake: FakeTransport, log: &mpsc::UnboundedSender<String>) -> Watched {
    Watched { fake, log: log.clone() }
}

fn stalling(fake: FakeTransport, log: &mpsc::UnboundedSender<String>) -> FakeTransport {
    let log = log.clone();
    fake.script(&REQUESTS, move |stdin, _| stall(log.clone(), stdin))
}

async fn stall(log: mpsc::UnboundedSender<String>, mut stdin: BoxStream<'static, Stdin>) -> End {
    if let Some(Stdin::Text(line)) = stdin.next().await {
        log.send(said(&line)).expect("the test reads the log");
    }
    time::sleep(Duration::from_secs(3600)).await;
    Ok(exit(0, ""))
}

fn answering_and_typing(log: &mpsc::UnboundedSender<String>, starts: &Starts) -> Arc<dyn Transport> {
    Arc::new(typing(answering(FakeTransport::default(), log, starts), log, starts, EACH))
}

mod reconnecting {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn cancels_the_command_in_progress_and_skips_what_was_queued_before_it() {
        let ((log, mut said), (new_log, mut new_said)) = (heard(), heard());
        let new_starts = Starts::default();
        let old = typing(stalling(FakeTransport::default(), &log), &log, &Starts::default(), EACH);
        let Started { queue, mut notices, .. } = start(watched(old, &log));
        queue.input(5, key("a"));
        let hung = queue.ask(words(&["hang"]), None);
        let queued = queue.ask(words(&["status"]), None);
        queue.input(5, key("b"));
        queue.scroll(5, 3);
        settled().await;
        queue.reconnect(answering_and_typing(&new_log, &new_starts));
        queue.input(5, key("c"));
        let focus = queue.ask(words(&["focus"]), None);
        assert_eq!((hung.await, queued.await), (Err(Error::Unanswered), Ok(Outcome::Skipped)));
        assert_eq!(focus.await, Ok(done("focus")));
        assert_eq!(said.taken(), [&["typed 5 a", "hang"][..], &CANCELLED].concat());
        assert_eq!(new_said.taken(), ["typed 5 c", "focus"]);
        assert_eq!((new_starts.count(), notices.taken()), (2, vec![]));
    }

    #[tokio::test(start_paused = true)]
    async fn skips_a_command_still_waiting_for_the_keys_before_it() {
        let ((log, mut said), (new_log, mut new_said)) = (heard(), heard());
        let old =
            typing(answering(FakeTransport::default(), &log, &Starts::default()), &log, &Starts::default(), SILENT);
        let Started { queue, mut notices, .. } = start(watched(old, &log));
        queue.input(5, key("a"));
        let waiting = queue.ask(words(&["status"]), None);
        settled().await;
        queue.reconnect(answering_and_typing(&new_log, &Starts::default()));
        let focus = queue.ask(words(&["focus"]), None);
        assert_eq!((waiting.await, focus.await), (Ok(Outcome::Skipped), Ok(done("focus"))));
        assert_eq!(said.taken(), ["typed 5 a", "cancelled input --follow --json -"]);
        assert_eq!((new_said.taken(), notices.taken()), (vec!["focus".to_owned()], vec![]));
    }

    #[tokio::test(start_paused = true)]
    async fn closes_both_idle_processes_without_a_warning() {
        let ((log, mut said), (new_log, mut new_said)) = (heard(), heard());
        let Started { queue, mut notices, .. } = start(watched(
            typing(answering(FakeTransport::default(), &log, &Starts::default()), &log, &Starts::default(), EACH),
            &log,
        ));
        queue.input(5, key("a"));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        queue.reconnect(answering_and_typing(&new_log, &Starts::default()));
        settled().await;
        assert_eq!(said.taken(), [&["typed 5 a", "status"][..], &CANCELLED].concat());
        assert_eq!(notices.taken(), []);
        assert_eq!(queue.ask(words(&["focus"]), None).await, Ok(done("focus")));
        assert_eq!(new_said.taken(), ["focus"]);
    }

    #[tokio::test]
    async fn tries_a_cli_that_lacked_both_processes_again_on_the_new_transport() {
        let ((log, _said), (new_log, mut new_said)) = (heard(), heard());
        let (starts, new_starts) = (Starts::default(), Starts::default());
        let old = refusing(
            FakeTransport::default().answer(&["status"], &["once"], &[], 0),
            Ok(exit(2, NO_REQUESTS_COMMAND)),
            &starts,
        );
        let old = typing_once(unstarted(old, Ok(exit(2, NO_FOLLOW_FLAG)), false, &starts), &log, &[5]);
        let Started { queue, mut notices, .. } = start(old);
        for code in ["a", "b"] {
            queue.input(5, key(code));
            assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("once")));
        }
        assert_eq!(starts.count(), 2);
        queue.reconnect(answering_and_typing(&new_log, &new_starts));
        queue.input(5, key("c"));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!(new_said.taken(), ["typed 5 c", "status"]);
        assert_eq!((new_starts.count(), notices.taken()), (2, vec![]));
    }
}
