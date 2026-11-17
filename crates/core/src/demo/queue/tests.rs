use std::sync::{Arc, PoisonError};
use std::time::Duration;

use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use rstest::rstest;
use serde_json::json;
use tokio::sync::mpsc;
use tokio::time;

use super::*;
use crate::cli::paste_file::{self, Pasted};
use crate::cli::{keys, last_message, mark};
use crate::demo::DemoTransport;
use crate::queue::{Outcome, Queue};
use crate::transport::Transport;

const ASKING: u64 = 13;

#[derive(Clone, Default)]
struct Seen {
    demo: DemoTransport,
    runs: Arc<Mutex<Vec<Vec<String>>>>,
}

impl Seen {
    fn runs(&self) -> Vec<String> {
        self.runs.lock().unwrap_or_else(PoisonError::into_inner).iter().map(|argv| argv.join(" ")).collect()
    }
}

impl Transport for Seen {
    fn run(
        &self,
        argv: Vec<String>,
        stdin: BoxStream<'static, Stdin>,
        out: OutSink,
    ) -> BoxFuture<'_, Result<Exit, Error>> {
        self.runs.lock().unwrap_or_else(PoisonError::into_inner).push(argv.clone());
        self.demo.run(argv, stdin, out).boxed()
    }
}

struct Started {
    queue: Queue,
    seen: Seen,
    notices: mpsc::UnboundedReceiver<Error>,
}

impl Started {
    fn notices(&mut self) -> Vec<Error> {
        std::iter::from_fn(|| self.notices.try_recv().ok()).collect()
    }
}

fn started() -> Started {
    let seen = Seen::default();
    let (notice, notices) = mpsc::unbounded_channel();
    let on_error = move |error| notice.send(error).expect("the test reads the notices");
    let (queue, worker) = Queue::new(Arc::new(seen.clone()), on_error);
    tokio::spawn(worker);
    Started { queue, seen, notices }
}

fn keys_one() -> Vec<String> {
    keys::args(ASKING, &["1"]).expect("1 is a key")
}

fn sent() -> (Vec<String>, Option<Stdin>) {
    (last_message::send_args(4), Some(Stdin::Text("Use the sandbox keys".to_owned())))
}

mod the_requests {
    use super::*;

    #[rstest]
    #[case::a_message(sent())]
    #[case::a_forced_message((last_message::forced_send_args(4), Some(Stdin::Text("Go on".to_owned()))))]
    #[case::a_key((keys_one(), None))]
    #[case::a_review((mark::reviewed_args(17), None))]
    #[tokio::test(start_paused = true)]
    async fn are_confirmed_by_the_demo(#[case] (args, stdin): (Vec<String>, Option<Stdin>)) {
        let mut started = started();

        let outcome = started.queue.ask(args, stdin).await;

        assert!(matches!(outcome, Ok(Outcome::Done(_))), "{outcome:?}");
        assert_eq!(started.notices(), Vec::new());
    }

    #[tokio::test(start_paused = true)]
    async fn go_through_one_requests_follow_without_falling_back() {
        let started = started();
        let (args, stdin) = sent();

        started.queue.ask(args, stdin).await.expect("the message is sent");
        started.queue.ask(keys_one(), None).await.expect("the key is pressed");
        started.queue.ask(mark::reviewed_args(17), None).await.expect("the agent is reviewed");

        assert_eq!(started.seen.runs(), vec!["requests --follow -".to_owned()]);
    }

    #[tokio::test(start_paused = true)]
    async fn refuse_what_the_demo_does_not_know() {
        let started = started();

        let outcome = started.queue.ask(vec!["reboot".to_owned()], None).await;

        assert!(matches!(outcome, Err(Error::Failed { code: 2, .. })), "{outcome:?}");
    }
}

mod the_input {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn is_typed_through_input_follow_and_confirmed() {
        let mut started = started();

        started.queue.input(4, json!({ "text": "yes" }));
        started.queue.input(4, json!({ "key": { "code": "enter" } }));
        started.queue.ask(mark::reviewed_args(17), None).await.expect("the typist settled first");
        time::sleep(Duration::from_secs(30)).await;

        assert_eq!(started.notices(), Vec::new());
        assert_eq!(started.seen.runs(), vec!["input --follow --json -".to_owned(), "requests --follow -".to_owned()]);
    }
}

mod an_image {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn is_accepted_with_its_name_and_size() {
        let started = started();
        let args = paste_file::args(4, "receipt.jpg");

        let outcome = started.queue.ask(args, Some(Stdin::Bytes(vec![7; 1024]))).await;

        let Ok(Outcome::Done(printed)) = outcome else { panic!("the image is pasted: {outcome:?}") };
        let pasted: Pasted = serde_json::from_str(&printed).expect("a pasted image");
        assert_eq!((pasted.pane, pasted.bytes), (4, 1024));
        assert!(pasted.path.starts_with("/Users/demo/") && pasted.path.ends_with("/receipt.jpg"), "{}", pasted.path);
    }
}
