use std::time::Duration;

use futures_util::{FutureExt, SinkExt};
use rstest::rstest;
use serde_json::Value;
use tokio::time;

use super::*;
use crate::cli::status::fixtures::VIEW;
use crate::transport::fake::FakeTransport;
use crate::transport::{Exit, Out, OutSink};

const ARGV: [&str; 7] = ["view", "--tab", "3", "--cols", "120", "--rows", "40"];
const SIZE: Size = Size { cols: 120, rows: 40 };
const GRANTED: Verdict = Verdict::Granted { view: 41, projection: 3 };
const REFUSAL: &str = "view 41 cannot size a tab now: it has no size";

async fn within<T>(work: impl Future<Output = T>) -> T {
    time::timeout(Duration::from_secs(5), work).await.expect("it gets there within five seconds")
}

fn line(edit: fn(&mut Value)) -> String {
    let mut line: Value = serde_json::from_str(VIEW).expect("the view fixture");
    edit(&mut line);
    line.to_string()
}

fn unchanged(_: &mut Value) {}

fn without_control(line: &mut Value) {
    line["view"]["controls"] = Value::Bool(false);
}

fn on_another_tab(line: &mut Value) {
    line["view"]["shown"]["tab"] = Value::from(9);
}

fn on_no_tab(line: &mut Value) {
    line["view"]["shown"]["tab"] = Value::Null;
}

fn raised(line: &mut Value) {
    line["view"]["projection"] = Value::from(4);
}

fn on_another_tab_without_control(line: &mut Value) {
    on_another_tab(line);
    without_control(line);
}

fn inactive_without_control(line: &mut Value) {
    line["view"]["active"] = Value::Bool(false);
    without_control(line);
}

fn unsized_without_control(line: &mut Value) {
    line["view"]["rows"] = Value::Null;
    without_control(line);
}

fn claim(fake: FakeTransport) -> Claim {
    Claim::open(Arc::new(fake), 3, SIZE)
}

fn printing(lines: &[String]) -> Claim {
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    claim(FakeTransport::default().open(&ARGV, &lines))
}

async fn verdicts(mut claim: Claim, count: usize) -> Vec<Verdict> {
    let mut verdicts = Vec::new();
    for _ in 0..count {
        verdicts.push(within(claim.next()).await);
    }
    verdicts
}

mod a_claim {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn is_granted_by_the_first_line_of_the_view_launched_on_the_tab_at_its_size() {
        assert_eq!(verdicts(printing(&[line(unchanged)]), 1).await, [GRANTED]);
    }

    #[tokio::test(start_paused = true)]
    async fn hands_the_projection_of_each_later_line() {
        assert_eq!(
            verdicts(printing(&[line(unchanged), line(raised), line(unchanged)]), 3).await,
            [GRANTED, Verdict::Projection(4), Verdict::Projection(3)]
        );
    }

    #[rstest]
    #[case::the_view_lost_control(without_control, Verdict::TakenBack)]
    #[case::the_view_settled_on_another_tab(on_another_tab, Verdict::MovedAway)]
    #[case::the_view_shows_no_tab(on_no_tab, Verdict::MovedAway)]
    #[case::moved_away_without_control(on_another_tab_without_control, Verdict::MovedAway)]
    #[case::an_inactive_view_lost_control(inactive_without_control, Verdict::Projection(3))]
    #[case::a_view_without_a_size_lost_control(unsized_without_control, Verdict::Projection(3))]
    #[tokio::test(start_paused = true)]
    async fn judges_each_line_after_the_grant(#[case] edit: fn(&mut Value), #[case] verdict: Verdict) {
        assert_eq!(verdicts(printing(&[line(unchanged), line(edit)]), 2).await, [GRANTED, verdict]);
    }

    #[rstest]
    #[case::taken_back(without_control, Verdict::TakenBack)]
    #[case::on_another_tab(on_another_tab, Verdict::MovedAway)]
    #[tokio::test(start_paused = true)]
    async fn is_not_granted_by_a_first_line_that_already_fails(#[case] edit: fn(&mut Value), #[case] verdict: Verdict) {
        assert_eq!(verdicts(printing(&[line(edit), line(unchanged)]), 2).await, [verdict, GRANTED]);
    }
}

mod a_refusal {
    use super::*;

    #[rstest]
    #[case::not_json("{\"view\": ")]
    #[case::not_a_view_line("{\"view\": 41}")]
    #[tokio::test(start_paused = true)]
    async fn comes_from_a_line_that_is_not_a_view_line(#[case] text: &str) {
        let verdict = within(claim(FakeTransport::default().open(&ARGV, &[text])).next()).await;
        assert!(matches!(verdict, Verdict::Refused(Error::Json(_))), "{verdict:?}");
    }

    #[rstest]
    #[case::the_view_cannot_size_the_tab(1, REFUSAL, Error::Failed { code: 1, stderr: REFUSAL.to_owned() })]
    #[case::the_tab_is_not_found(1, "no tab 3", Error::Failed { code: 1, stderr: "no tab 3".to_owned() })]
    #[case::the_view_ended_before_its_first_line(0, "", Error::StreamEnded)]
    #[tokio::test(start_paused = true)]
    async fn carries_what_cornercase_printed_when_the_view_ends(
        #[case] code: i32,
        #[case] stderr: &str,
        #[case] error: Error,
    ) {
        let fake = FakeTransport::default().answer(&ARGV, &[], &[stderr], code);
        assert_eq!(within(claim(fake).next()).await, Verdict::Refused(error));
    }
}

mod an_end {
    use super::*;

    const RESTARTED: &str = "boot changed";

    fn ends(fake: FakeTransport) -> FakeTransport {
        fake.answer(&ARGV, &[], &[RESTARTED], 4)
    }

    fn lost(fake: FakeTransport) -> FakeTransport {
        fake.fail(&ARGV, Error::Dropped)
    }

    #[rstest]
    #[case::the_server_restarted(ends, Ended::Failed(Error::StaleBoot(RESTARTED.to_owned())))]
    #[case::the_connection_was_lost(lost, Ended::Lost(Error::Dropped))]
    #[tokio::test(start_paused = true)]
    async fn before_the_first_line_is_no_refusal_when_it_says_nothing_of_the_tab(
        #[case] fake: fn(FakeTransport) -> FakeTransport,
        #[case] ended: Ended,
    ) {
        assert_eq!(within(claim(fake(FakeTransport::default())).next()).await, Verdict::Ended(ended));
    }

    #[rstest]
    #[case::exited(0, &[], Ended::Exited)]
    #[case::failed(1, &[REFUSAL], Ended::Failed(Error::Failed { code: 1, stderr: REFUSAL.to_owned() }))]
    #[case::stale(4, &[RESTARTED], Ended::Failed(Error::StaleBoot(RESTARTED.to_owned())))]
    #[tokio::test(start_paused = true)]
    async fn after_the_grant_follows_its_lines_and_is_told_once(
        #[case] code: i32,
        #[case] stderr: &[&str],
        #[case] ended: Ended,
    ) {
        let fixture = line(unchanged);
        let fake = FakeTransport::default().answer(&ARGV, &[&fixture], stderr, code);
        let mut claim = claim(fake);
        assert_eq!(within(claim.next()).await, GRANTED);
        assert_eq!(within(claim.next()).await, Verdict::Ended(ended));
        assert_eq!(claim.next().now_or_never(), None);
    }

    async fn lost_after(mut out: OutSink, line: String) -> Result<Exit, Error> {
        out.send(Out::Stdout(line)).await.map_err(|_| Error::OutputClosed)?;
        Err(Error::Dropped)
    }

    #[tokio::test(start_paused = true)]
    async fn after_the_grant_comes_from_a_lost_connection_too() {
        let fixture = line(unchanged);
        let fake = FakeTransport::default().script(&ARGV, move |_, out| lost_after(out, fixture.clone()));
        assert_eq!(verdicts(claim(fake), 2).await, [GRANTED, Verdict::Ended(Ended::Lost(Error::Dropped))]);
    }
}

mod a_fit {
    use std::iter;

    use tokio::sync::{mpsc, oneshot};
    use tokio::task::JoinHandle;

    use super::*;
    use crate::cli::surface::fixtures::FRAME;

    const SURFACE: [&str; 4] = ["surface", "--view", "41", "--patches"];

    type Painted = Result<(u64, Option<u64>), Error>;

    fn frame(tab: u64, projection: u64) -> String {
        let mut frame: Value = serde_json::from_str(FRAME).expect("the frame fixture");
        (frame["view"], frame["tab"], frame["projection"]) = (41.into(), tab.into(), projection.into());
        frame.to_string()
    }

    fn idle_queue() -> Queue {
        Queue::new(Arc::new(FakeTransport::default()), |_| {}).0
    }

    fn handing(fake: FakeTransport, argv: &[&str], outputs: &mpsc::UnboundedSender<OutSink>) -> FakeTransport {
        let outputs = outputs.clone();
        fake.script(argv, move |_, out| {
            outputs.send(out).expect("the test reads each output");
            future::pending()
        })
    }

    fn painting(painted: mpsc::UnboundedSender<Painted>) -> impl FnMut(&Grid, Result<Changes, Error>) {
        move |grid, changes| {
            let seen = changes.map(|_| (grid.tab, grid.projection));
            painted.send(seen).expect("the test reads what was painted");
        }
    }

    struct Host {
        launched: mpsc::UnboundedReceiver<String>,
        outputs: mpsc::UnboundedReceiver<OutSink>,
        painted: mpsc::UnboundedReceiver<Painted>,
        moved_on: oneshot::Sender<()>,
        following: JoinHandle<Fitted>,
    }

    impl Host {
        fn fitting() -> Self {
            let (outputs, handed) = mpsc::unbounded_channel();
            let fake = handing(handing(FakeTransport::default(), &ARGV, &outputs), &SURFACE, &outputs);
            let (transport, launched) = fake.launched();
            let (moved_on, moved) = oneshot::channel();
            let moved = async move { moved.await.ok().unwrap_or_default() };
            let (painted, seen) = mpsc::unbounded_channel();
            let claim = Claim::open(transport.clone(), 3, SIZE);
            let following = tokio::spawn(follow_fitted(claim, transport, idle_queue(), moved, painting(painted)));
            Self { launched, outputs: handed, painted: seen, moved_on, following }
        }

        async fn granted() -> (Self, OutSink, OutSink) {
            let mut host = Self::fitting();
            let mut view = host.output().await;
            say(&mut view, line(unchanged)).await;
            let surface = host.output().await;
            (host, view, surface)
        }

        async fn output(&mut self) -> OutSink {
            within(self.outputs.recv()).await.expect("a process was launched")
        }

        async fn painted(&mut self) -> Option<Painted> {
            within(self.painted.recv()).await
        }

        async fn fitted(&mut self) -> Fitted {
            within(&mut self.following).await.expect("the fit ends")
        }

        fn launched(&mut self) -> Vec<String> {
            iter::from_fn(|| self.launched.try_recv().ok()).collect()
        }
    }

    async fn say(out: &mut OutSink, line: String) {
        out.send(Out::Stdout(line)).await.expect("the process is still followed");
    }

    async fn released(out: &mut OutSink) -> bool {
        out.send(Out::Stdout(line(unchanged))).await.is_err()
    }

    async fn fitted(fake: FakeTransport, moved_on: impl Future<Output = ()>) -> (Fitted, Vec<String>) {
        let (transport, mut launched) = fake.launched();
        let claim = Claim::open(transport.clone(), 3, SIZE);
        let fitted = within(follow_fitted(claim, transport, idle_queue(), moved_on, |_, _| {})).await;
        (fitted, iter::from_fn(|| launched.try_recv().ok()).collect())
    }

    #[tokio::test(start_paused = true)]
    async fn launches_the_surface_of_the_view_only_after_its_first_line() {
        let mut host = Host::fitting();
        let mut view = host.output().await;
        assert_eq!(host.launched(), [ARGV.join(" ")]);
        say(&mut view, line(unchanged)).await;
        let mut surface = host.output().await;
        assert_eq!(host.launched(), [SURFACE.join(" ")]);
        say(&mut surface, frame(3, 3)).await;
        assert_eq!(host.painted().await, Some(Ok((3, Some(3)))));
    }

    #[tokio::test(start_paused = true)]
    async fn paints_no_frame_behind_the_projection_the_view_last_told() {
        let (mut host, mut view, mut surface) = Host::granted().await;
        say(&mut surface, frame(3, 2)).await;
        say(&mut surface, frame(3, 3)).await;
        assert_eq!(host.painted().await, Some(Ok((3, Some(3)))));
        say(&mut view, line(raised)).await;
        say(&mut surface, frame(3, 3)).await;
        say(&mut surface, frame(3, 4)).await;
        assert_eq!(host.painted().await, Some(Ok((3, Some(4)))));
    }

    #[tokio::test(start_paused = true)]
    async fn paints_no_frame_of_another_tab() {
        let (mut host, _view, mut surface) = Host::granted().await;
        say(&mut surface, frame(9, 3)).await;
        say(&mut surface, frame(3, 3)).await;
        assert_eq!(host.painted().await, Some(Ok((3, Some(3)))));
    }

    #[tokio::test(start_paused = true)]
    async fn tells_of_a_line_that_is_no_surface_and_goes_on() {
        let (mut host, _view, mut surface) = Host::granted().await;
        say(&mut surface, "{\"boot\":".to_owned()).await;
        say(&mut surface, frame(3, 3)).await;
        let warned = Err(Error::Json("the JSON ends too early at line 1 column 8".to_owned()));
        assert_eq!(host.painted().await, Some(warned));
        assert_eq!(host.painted().await, Some(Ok((3, Some(3)))));
    }

    #[rstest]
    #[case::taken_back(without_control, Fitted::TakenBack)]
    #[case::moved_away(on_another_tab, Fitted::MovedAway)]
    #[tokio::test(start_paused = true)]
    async fn lets_go_of_both_processes_when_a_line_ends_it(#[case] edit: fn(&mut Value), #[case] fitted: Fitted) {
        let (mut host, mut view, mut surface) = Host::granted().await;
        say(&mut view, line(edit)).await;
        assert_eq!(host.fitted().await, fitted);
        assert!(released(&mut view).await && released(&mut surface).await);
    }

    #[tokio::test(start_paused = true)]
    async fn lets_go_of_both_processes_when_the_store_moved_on() {
        let (Host { moved_on, following, .. }, mut view, mut surface) = Host::granted().await;
        moved_on.send(()).expect("the fit is still followed");
        assert_eq!(within(following).await.expect("the fit ends"), Fitted::MovedOn);
        assert!(released(&mut view).await && released(&mut surface).await);
    }

    #[rstest]
    #[case::taken_back(without_control, Fitted::TakenBack)]
    #[case::moved_away(on_another_tab, Fitted::MovedAway)]
    #[tokio::test(start_paused = true)]
    async fn launches_no_surface_when_the_first_line_ends_it(#[case] edit: fn(&mut Value), #[case] fitted: Fitted) {
        let fake = FakeTransport::default().open(&ARGV, &[&line(edit)]);
        assert_eq!(self::fitted(fake, future::pending()).await, (fitted, vec![ARGV.join(" ")]));
    }

    #[tokio::test(start_paused = true)]
    async fn launches_no_surface_when_the_view_is_refused() {
        let fake = FakeTransport::default().answer(&ARGV, &[], &[REFUSAL], 1);
        let refused = Fitted::Refused(Error::Failed { code: 1, stderr: REFUSAL.to_owned() });
        assert_eq!(fitted(fake, future::pending()).await, (refused, vec![ARGV.join(" ")]));
    }

    #[tokio::test(start_paused = true)]
    async fn launches_no_surface_when_the_store_moved_on_before_the_first_line() {
        let fake = FakeTransport::default().open(&ARGV, &[]);
        assert_eq!(fitted(fake, future::ready(())).await, (Fitted::MovedOn, vec![ARGV.join(" ")]));
    }

    #[tokio::test(start_paused = true)]
    async fn ends_with_the_surface_of_the_view() {
        let fake = FakeTransport::default().open(&ARGV, &[&line(unchanged)]).answer(&SURFACE, &[], &[], 0);
        let launched = vec![ARGV.join(" "), SURFACE.join(" ")];
        assert_eq!(fitted(fake, future::pending()).await, (Fitted::Ended(Ended::Exited), launched));
    }
}
