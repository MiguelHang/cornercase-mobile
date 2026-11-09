use std::future;
use std::time::Duration;

use futures_util::SinkExt;
use futures_util::stream::BoxStream;
use rstest::rstest;
use serde_json::Value;
use tokio::time;

use super::*;
use crate::cli::surface::fixtures::{FRAME, PATCH};
use crate::transport::fake::FakeTransport;
use crate::transport::{Out, OutSink, Stdin};

const ARGV: [&str; 8] = ["surface", "--tab", "12", "--cols", "80", "--rows", "24", "--patches"];
const SIZE: Size = Size { cols: 80, rows: 24 };

struct Launched {
    fake: FakeTransport,
    argv: mpsc::UnboundedSender<String>,
}

impl Transport for Launched {
    fn run(
        &self,
        argv: Vec<String>,
        stdin: BoxStream<'static, Stdin>,
        out: OutSink,
    ) -> BoxFuture<'_, Result<Exit, Error>> {
        self.argv.send(argv.join(" ")).expect("the test reads what was launched");
        self.fake.run(argv, stdin, out)
    }
}

fn launched(fake: FakeTransport) -> (Arc<dyn Transport>, mpsc::UnboundedReceiver<String>) {
    let (argv, heard) = mpsc::unbounded_channel();
    (Arc::new(Launched { fake, argv }), heard)
}

fn idle_queue() -> Queue {
    Queue::new(Arc::new(FakeTransport::default()), |_| {}).0
}

fn live(fake: FakeTransport) -> Live {
    Live::open(Arc::new(fake), idle_queue(), 12, SIZE)
}

fn printing(lines: &[&str]) -> Live {
    live(FakeTransport::default().answer(&ARGV, lines, &[], 0))
}

async fn steps(mut live: Live) -> Vec<Step> {
    let mut steps = Vec::new();
    loop {
        let step = live.next().await;
        let over = matches!(step, Step::Ended(_));
        steps.push(step);
        if over {
            return steps;
        }
    }
}

fn one_line(json: &str) -> String {
    serde_json::from_str::<Value>(json).expect("a surface fixture").to_string()
}

fn patched(edit: impl FnOnce(&mut Value)) -> String {
    let mut patch: Value = serde_json::from_str(PATCH).expect("the patch fixture");
    edit(&mut patch);
    patch.to_string()
}

fn on_another_base() -> String {
    patched(|patch| patch["base"] = Value::from(99))
}

fn without_copy() -> String {
    patched(|patch| patch["panes"][0]["copy"] = Value::Null)
}

fn changed(replaced: bool, panes: &[(u64, &[u16])]) -> Step {
    let rows = panes.iter().map(|(pane, rows)| (*pane, rows.iter().copied().collect())).collect();
    Step::Changed(Changes { replaced, rows })
}

fn whole_frame() -> Step {
    changed(true, &[(13, &[0, 1, 2, 3, 4]), (14, &[0])])
}

mod open {
    use super::*;

    #[tokio::test]
    async fn launches_only_the_surface_of_the_tab_at_its_size_with_patches() {
        let (transport, mut argv) = launched(FakeTransport::default().answer(&ARGV, &[], &[], 0));
        let live = Live::open(transport, idle_queue(), 12, SIZE);
        assert_eq!(steps(live).await, [Step::Ended(Ended::Exited)]);
        assert_eq!(argv.recv().await, Some(ARGV.join(" ")));
        assert!(argv.try_recv().is_err());
    }

    #[tokio::test]
    async fn stops_the_stream_when_dropped() {
        let (closed, mut heard) = mpsc::unbounded_channel();
        let fake = FakeTransport::default().script(&ARGV, move |_, out| {
            tokio::spawn(print_until_closed(out, closed.clone()));
            future::pending()
        });
        let mut live = live(fake);
        assert_eq!(live.next().await, whole_frame());
        drop(live);
        assert_eq!(heard.recv().await, Some(()));
    }

    async fn print_until_closed(mut out: OutSink, closed: mpsc::UnboundedSender<()>) {
        while out.send(Out::Stdout(one_line(FRAME))).await.is_ok() {
            time::sleep(Duration::from_millis(1)).await;
        }
        closed.send(()).expect("the test waits for the stream to close");
    }
}

mod next {
    use super::*;

    #[tokio::test]
    async fn hands_every_row_of_a_complete_frame_as_replaced() {
        let live = printing(&[&one_line(FRAME)]);
        assert_eq!(steps(live).await, [whole_frame(), Step::Ended(Ended::Exited)]);
    }

    #[tokio::test]
    async fn hands_only_the_rows_a_patch_changed() {
        let live = printing(&[&one_line(FRAME), &without_copy()]);
        let patched = changed(false, &[(13, &[3, 4])]);
        assert_eq!(steps(live).await, [whole_frame(), patched, Step::Ended(Ended::Exited)]);
    }

    #[tokio::test]
    async fn keeps_the_grid_the_lines_built() {
        let mut live = printing(&[&one_line(FRAME), &one_line(PATCH)]);
        live.next().await;
        live.next().await;
        assert_eq!((live.grid().surface, live.grid().revision), (2, 4));
    }

    #[tokio::test]
    async fn warns_of_a_broken_line_and_goes_on() {
        let live = printing(&["{\"boot\":", &one_line(FRAME)]);
        let warned = Step::Warned(Error::Json("the JSON ends too early at line 1 column 8".to_owned()));
        assert_eq!(steps(live).await, [warned, whole_frame(), Step::Ended(Ended::Exited)]);
    }

    #[rstest]
    #[case::exited(Ok(Exit { code: 0, stderr: String::new() }), Ended::Exited)]
    #[case::failed(
        Ok(Exit { code: 1, stderr: "no tab 12".to_owned() }),
        Ended::Failed(Error::Failed { code: 1, stderr: "no tab 12".to_owned() })
    )]
    #[case::stale_boot(
        Ok(Exit { code: 4, stderr: "another boot".to_owned() }),
        Ended::Failed(Error::StaleBoot("another boot".to_owned()))
    )]
    #[case::lost(Err(Error::Connect("refused".to_owned())), Ended::Lost(Error::Connect("refused".to_owned())))]
    #[tokio::test]
    async fn says_how_the_stream_ended(#[case] end: End, #[case] ended: Ended) {
        let live = live(FakeTransport::default().script(&ARGV, move |_, _| future::ready(end.clone())));
        assert_eq!(steps(live).await, [Step::Ended(ended)]);
    }
}

mod a_patch_on_another_base {
    use super::*;

    fn redrawn(redraws: &mpsc::UnboundedSender<String>) -> FakeTransport {
        let redraws = redraws.clone();
        let fake = FakeTransport::default().answer(&["status"], &[], &[], 0);
        fake.script(&["redraw", "--surface", "2"], move |_, _| {
            redraws.send("redraw --surface 2".to_owned()).expect("the test reads the redraws");
            future::ready(Ok(Exit { code: 0, stderr: String::new() }))
        })
    }

    #[tokio::test]
    async fn asks_once_for_a_redraw_and_hands_nothing_until_a_complete_frame() {
        let (sender, mut redraws) = mpsc::unbounded_channel();
        let (queue, worker) = Queue::new(Arc::new(redrawn(&sender)), |_| {});
        tokio::spawn(worker);
        let lines = [one_line(FRAME), on_another_base(), on_another_base(), one_line(PATCH), one_line(FRAME)];
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        let live =
            Live::open(Arc::new(FakeTransport::default().answer(&ARGV, &lines, &[], 0)), queue.clone(), 12, SIZE);
        assert_eq!(steps(live).await, [whole_frame(), whole_frame(), Step::Ended(Ended::Exited)]);
        queue.ask(vec!["status".to_owned()], None).await.expect("the queue ran what came before");
        assert_eq!(redraws.try_recv().as_deref(), Ok("redraw --surface 2"));
        assert!(redraws.try_recv().is_err());
    }
}
