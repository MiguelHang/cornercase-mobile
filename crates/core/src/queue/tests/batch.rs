use futures_util::FutureExt;
use futures_util::future::BoxFuture;

use super::input::{EACH, key, typing};
use super::*;
use crate::cli::paste_file;

const PASTE: [&str; 6] = ["paste-file", "--pane", "5", "--name=shot.png", "--json", "-"];

#[derive(Clone, Copy)]
enum Step {
    Keyed(&'static str, &'static str),
    Plain(&'static str),
    Scrolled(u64, i64),
}

use Step::{Keyed, Plain, Scrolled};

fn queued(queue: &Queue, steps: &[Step]) -> Vec<BoxFuture<'static, Result<Outcome, Error>>> {
    let mut asked = Vec::new();
    for &step in steps {
        match step {
            Keyed(key, word) => asked.push(queue.ask_coalesced(key, words(&[word])).boxed()),
            Plain(word) => asked.push(queue.ask(words(&[word]), None).boxed()),
            Scrolled(pane, rows) => queue.scroll(pane, rows),
        }
    }
    asked
}

async fn answers(asked: Vec<BoxFuture<'static, Result<Outcome, Error>>>) -> Vec<Outcome> {
    let mut answers = Vec::new();
    for answer in asked {
        answers.push(answer.await.expect("an answer"));
    }
    answers
}

fn pasting(fake: FakeTransport, log: &mpsc::UnboundedSender<String>) -> FakeTransport {
    let log = log.clone();
    fake.script(&PASTE, move |stdin, out| paste(log.clone(), stdin, out))
}

async fn paste(log: mpsc::UnboundedSender<String>, stdin: BoxStream<'static, Stdin>, mut out: OutSink) -> End {
    let read: Vec<Stdin> = stdin.collect().await;
    log.send(format!("once paste-file < {read:?}")).expect("the test reads the log");
    out.send(Out::Stdout("pasted".to_owned())).await.map_err(|Closed| Error::OutputClosed)?;
    Ok(exit(0, ""))
}

mod coalesce {
    use super::*;

    #[rstest]
    #[case::one_key(
        &[Keyed("view", "a"), Keyed("view", "b"), Keyed("view", "c")],
        &["c"],
        vec![Outcome::Skipped, Outcome::Skipped, done("c")],
    )]
    #[case::two_runs_cut_by_a_command(
        &[Keyed("view", "a"), Keyed("view", "b"), Plain("focus"), Keyed("view", "c"), Keyed("view", "d")],
        &["b", "focus", "d"],
        vec![Outcome::Skipped, done("b"), done("focus"), Outcome::Skipped, done("d")],
    )]
    #[case::cut_by_another_key(
        &[Keyed("view", "a"), Keyed("fit", "b"), Keyed("view", "c")],
        &["a", "b", "c"],
        vec![done("a"), done("b"), done("c")],
    )]
    #[case::cut_by_a_scroll(
        &[Keyed("view", "a"), Scrolled(5, 1), Keyed("view", "b")],
        &["a", "scroll --pane 5 --up 1", "b"],
        vec![done("a"), done("b")],
    )]
    #[case::without_a_key(&[Plain("a"), Plain("a")], &["a", "a"], vec![done("a"), done("a")])]
    #[tokio::test]
    async fn runs_only_the_last_of_a_run_with_the_same_key(
        #[case] steps: &[Step],
        #[case] ran: &[&str],
        #[case] outcomes: Vec<Outcome>,
    ) {
        let (log, mut said) = heard();
        let Started { queue, mut notices, .. } = start(answering(FakeTransport::default(), &log, &Starts::default()));
        let asked = queued(&queue, steps);
        assert_eq!(answers(asked).await, outcomes);
        assert_eq!(said.taken(), ran);
        assert_eq!(notices.taken(), []);
    }

    #[tokio::test]
    async fn neither_runs_nor_reports_a_command_nobody_waits_for_that_was_replaced() {
        let (log, mut said) = heard();
        let Started { queue, mut notices, .. } = start(answering(FakeTransport::default(), &log, &Starts::default()));
        queue.coalesced("view", words(&["fail"]));
        queue.coalesced("view", words(&["status"]));
        assert_eq!(queue.ask(words(&["focus"]), None).await, Ok(done("focus")));
        assert_eq!(said.taken(), ["status", "focus"]);
        assert_eq!(notices.taken(), []);
    }
}

mod scroll {
    use super::*;

    #[rstest]
    #[case::up(&[Scrolled(5, 3), Scrolled(5, 2)], &["scroll --pane 5 --up 5"])]
    #[case::down(&[Scrolled(5, -3), Scrolled(5, 1)], &["scroll --pane 5 --down 2"])]
    #[case::back_where_it_was(&[Scrolled(5, 3), Scrolled(5, -3)], &[])]
    #[case::cut_by_another_pane(
        &[Scrolled(5, 1), Scrolled(6, 1), Scrolled(5, 1)],
        &["scroll --pane 5 --up 1", "scroll --pane 6 --up 1", "scroll --pane 5 --up 1"],
    )]
    #[case::cut_by_a_command(
        &[Scrolled(5, 1), Plain("focus"), Scrolled(5, 1)],
        &["scroll --pane 5 --up 1", "focus", "scroll --pane 5 --up 1"],
    )]
    #[tokio::test]
    async fn sends_the_net_rows_of_a_run_on_one_pane(#[case] steps: &[Step], #[case] sent: &[&str]) {
        let (log, mut said) = heard();
        let Started { queue, mut notices, .. } = start(answering(FakeTransport::default(), &log, &Starts::default()));
        let asked = queued(&queue, steps);
        answers(asked).await;
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        assert_eq!(said.taken(), [sent, &["status"]].concat());
        assert_eq!(notices.taken(), []);
    }
}

mod bytes {
    use super::*;

    #[tokio::test]
    async fn run_once_after_the_keys_before_them() {
        let (log, mut said) = heard();
        let starts = Starts::default();
        let fake = typing(answering(FakeTransport::default(), &log, &starts), &log, &starts, EACH);
        let Started { queue, mut notices, .. } = start(pasting(fake, &log));
        queue.input(5, key("a"));
        let pasted = queue.ask(paste_file::args(5, "shot.png"), Some(Stdin::Bytes(vec![0, 255])));
        queue.input(5, key("b"));
        assert_eq!(pasted.await, Ok(done("pasted")));
        assert_eq!(queue.ask(words(&["status"]), None).await, Ok(done("status")));
        let pasted = "once paste-file < [Bytes([0, 255])]";
        assert_eq!(said.taken(), ["typed 5 a", pasted, "typed 5 b", "status"]);
        assert_eq!(notices.taken(), []);
    }
}
