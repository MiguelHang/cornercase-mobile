use std::future;
use std::time::Duration;

use futures_util::SinkExt;
use rstest::rstest;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::time;

use super::*;
use crate::cli::surface::fixtures::{FRAME, PATCH};
use crate::follow::fake::{Said, exits, played};
use crate::follow::{End, Ended};
use crate::transport::fake::FakeTransport;
use crate::transport::{Exit, Out, OutSink};

const ARGV: [&str; 8] = ["surface", "--tab", "12", "--cols", "80", "--rows", "24", "--patches"];
const SIZE: Size = Size { cols: 80, rows: 24 };
const TAB_12: Source = Source::Tab { tab: 12, size: SIZE };
const VIEW_ARGV: [&str; 4] = ["surface", "--view", "41", "--patches"];
const VIEW_41: Source = Source::View { view: 41, tab: 12 };

async fn within<T>(work: impl Future<Output = T>) -> T {
    time::timeout(Duration::from_secs(5), work).await.expect("it gets there within five seconds")
}

fn idle_queue() -> Queue {
    Queue::new(Arc::new(FakeTransport::default()), |_| {}).0
}

fn live(fake: FakeTransport) -> Live {
    Live::open(Arc::new(fake), idle_queue(), TAB_12)
}

fn printing(lines: &[&str]) -> Live {
    live(FakeTransport::default().answer(&ARGV, lines, &[], 0))
}

async fn steps(mut live: Live) -> Vec<Step> {
    let mut steps = Vec::new();
    loop {
        let step = within(live.next()).await;
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

    #[tokio::test(start_paused = true)]
    async fn launches_only_the_surface_of_the_tab_at_its_size_with_patches() {
        let (transport, mut argv) = FakeTransport::default().answer(&ARGV, &[], &[], 0).launched();
        let live = Live::open(transport, idle_queue(), TAB_12);
        assert_eq!(steps(live).await, [Step::Ended(Ended::Exited)]);
        assert_eq!(within(argv.recv()).await, Some(ARGV.join(" ")));
        assert!(argv.try_recv().is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn launches_the_surface_of_a_view_with_patches() {
        let (transport, mut launched) = FakeTransport::default().answer(&VIEW_ARGV, &[], &[], 0).launched();
        let live = Live::open(transport, idle_queue(), VIEW_41);
        assert_eq!(steps(live).await, [Step::Ended(Ended::Exited)]);
        assert_eq!(within(launched.recv()).await, Some(VIEW_ARGV.join(" ")));
    }

    #[tokio::test(start_paused = true)]
    async fn stops_the_stream_when_dropped() {
        let (closed, mut heard) = mpsc::unbounded_channel();
        let fake = FakeTransport::default().script(&ARGV, move |_, out| {
            tokio::spawn(print_until_closed(out, closed.clone()));
            future::pending()
        });
        let mut live = live(fake);
        assert_eq!(within(live.next()).await, whole_frame());
        drop(live);
        assert_eq!(within(heard.recv()).await, Some(()));
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

    #[tokio::test(start_paused = true)]
    async fn hands_every_row_of_a_complete_frame_as_replaced() {
        let live = printing(&[&one_line(FRAME)]);
        assert_eq!(steps(live).await, [whole_frame(), Step::Ended(Ended::Exited)]);
    }

    #[tokio::test(start_paused = true)]
    async fn hands_only_the_rows_a_patch_changed() {
        let live = printing(&[&one_line(FRAME), &without_copy()]);
        let patched = changed(false, &[(13, &[3, 4])]);
        assert_eq!(steps(live).await, [whole_frame(), patched, Step::Ended(Ended::Exited)]);
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_grid_the_lines_built() {
        let mut live = printing(&[&one_line(FRAME), &one_line(PATCH)]);
        within(live.next()).await;
        within(live.next()).await;
        assert_eq!((live.grid().surface, live.grid().revision), (2, 4));
    }

    #[tokio::test(start_paused = true)]
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
    #[tokio::test(start_paused = true)]
    async fn says_how_the_stream_ended(#[case] end: End, #[case] ended: Ended) {
        let live = live(FakeTransport::default().script(&ARGV, move |_, _| future::ready(end.clone())));
        assert_eq!(steps(live).await, [Step::Ended(ended)]);
    }
}

mod a_view {
    use super::*;

    fn of_view(json: &str, edit: impl FnOnce(&mut Value)) -> String {
        let mut line: Value = serde_json::from_str(json).expect("a surface fixture");
        line["view"] = Value::from(41);
        edit(&mut line);
        line.to_string()
    }

    fn at(json: &str, projection: u64) -> String {
        of_view(json, |line| line["projection"] = Value::from(projection))
    }

    fn printing(lines: &[String]) -> Live {
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        let fake = FakeTransport::default().answer(&VIEW_ARGV, &lines, &[], 0);
        Live::open(Arc::new(fake), idle_queue(), VIEW_41)
    }

    #[rstest]
    #[case::a_patch_of_another_view(PATCH, |line: &mut Value| line["view"] = Value::from(40))]
    #[case::a_patch_of_another_tab(PATCH, |line: &mut Value| line["tab"] = Value::from(13))]
    #[case::a_frame_without_a_view(FRAME, |line: &mut Value| line["view"] = Value::Null)]
    #[case::a_frame_of_another_tab(FRAME, |line: &mut Value| line["tab"] = Value::from(13))]
    #[tokio::test(start_paused = true)]
    async fn does_not_apply_a_line_meant_for_another(#[case] json: &str, #[case] edit: fn(&mut Value)) {
        let mut live = printing(&[of_view(FRAME, |_| {}), of_view(json, edit)]);
        assert_eq!(within(live.next()).await, whole_frame());
        assert_eq!(within(live.next()).await, Step::Ended(Ended::Exited));
        assert_eq!((live.grid().tab, live.grid().revision), (12, 3));
    }

    #[tokio::test(start_paused = true)]
    async fn hands_nothing_of_a_frame_older_than_the_projection_it_saw() {
        let mut live = printing(&[at(FRAME, 2)]);
        live.saw_projection(3);
        assert_eq!(steps(live).await, [Step::Ended(Ended::Exited)]);
    }

    #[tokio::test(start_paused = true)]
    async fn hands_the_first_complete_frame_after_the_projection_rose_as_replaced() {
        let mut live = printing(&[at(FRAME, 2), at(&without_copy(), 2), at(FRAME, 3)]);
        assert_eq!(within(live.next()).await, whole_frame());
        live.saw_projection(3);
        assert_eq!(steps(live).await, [whole_frame(), Step::Ended(Ended::Exited)]);
    }
}

mod a_patch_on_another_base {
    use super::*;

    struct Redraws {
        queue: Queue,
        asked: mpsc::UnboundedReceiver<String>,
    }

    impl Redraws {
        fn answering(code: i32) -> Self {
            let (sender, asked) = mpsc::unbounded_channel();
            let fake = FakeTransport::default().answer(&["status"], &[], &[], 0);
            let fake = fake.script(&["redraw", "--surface", "2"], move |_, _| {
                sender.send("redraw --surface 2".to_owned()).expect("the test reads the redraws");
                future::ready(Ok(Exit { code, stderr: String::new() }))
            });
            let (queue, worker) = Queue::new(Arc::new(fake), |_| {});
            tokio::spawn(worker);
            Self { queue, asked }
        }

        async fn asked(mut self) -> usize {
            self.queue.ask(vec!["status".to_owned()], None).await.expect("the queue ran what came before");
            let mut asked = 0;
            while self.asked.try_recv().is_ok() {
                asked += 1;
            }
            asked
        }
    }

    #[tokio::test(start_paused = true)]
    async fn asks_once_for_a_redraw_and_hands_nothing_until_a_complete_frame() {
        let redraws = Redraws::answering(0);
        let lines = [one_line(FRAME), on_another_base(), on_another_base(), one_line(PATCH), one_line(FRAME)];
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        let fake = FakeTransport::default().answer(&ARGV, &lines, &[], 0);
        let live = Live::open(Arc::new(fake), redraws.queue.clone(), TAB_12);
        assert_eq!(steps(live).await, [whole_frame(), whole_frame(), Step::Ended(Ended::Exited)]);
        assert_eq!(redraws.asked().await, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn does_not_ask_again_for_patches_one_second_after_a_redraw() {
        let redraws = Redraws::answering(1);
        let patch = || Said::Line(on_another_base());
        let said = vec![Said::Line(one_line(FRAME)), patch(), Said::After(1), patch(), exits()];
        let fake = FakeTransport::default().script(&ARGV, move |_, out| played(out, said.clone()));
        let live = Live::open(Arc::new(fake), redraws.queue.clone(), TAB_12);
        assert_eq!(steps(live).await, [whole_frame(), Step::Ended(Ended::Exited)]);
        assert_eq!(redraws.asked().await, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn asks_again_when_patches_go_on_two_seconds_after_a_redraw_that_brought_nothing() {
        let redraws = Redraws::answering(1);
        let patch = || Said::Line(on_another_base());
        let said =
            vec![Said::Line(one_line(FRAME)), patch(), Said::After(1), patch(), Said::After(2), patch(), exits()];
        let fake = FakeTransport::default().script(&ARGV, move |_, out| played(out, said.clone()));
        let live = Live::open(Arc::new(fake), redraws.queue.clone(), TAB_12);
        assert_eq!(steps(live).await, [whole_frame(), Step::Ended(Ended::Exited)]);
        assert_eq!(redraws.asked().await, 2);
    }
}

mod a_terminal {
    use super::*;
    use crate::cli::status::fixtures::VIEW;
    use crate::follow::fake::{self, BOOT, Host, Watched, connected, dropped, dropped_for_a_while, session};

    const TAB: [&str; 8] = ["surface", "--tab", "3", "--cols", "80", "--rows", "24", "--patches"];
    const CLAIM: [&str; 7] = ["view", "--tab", "3", "--cols", "80", "--rows", "24"];

    fn drawn() -> Said {
        Said::Line(one_line(FRAME))
    }

    fn watching(sessions: Vec<Vec<Said>>, surfaces: Vec<Vec<Said>>) -> Host {
        Host::of(&TAB, sessions, surfaces)
    }

    struct OnScreen(Watched<Screen>);

    impl OnScreen {
        async fn after_a_minute(host: Host) -> Self {
            Self::for_a_minute(host, false).await
        }

        async fn fitted_for_a_minute(host: Host) -> Self {
            Self::for_a_minute(host, true).await
        }

        async fn for_a_minute(host: Host, fit: bool) -> Self {
            let terminal = Terminal { tab: 3, pane: None, size: SIZE, fit };
            Self(Watched::after_a_minute(host, move |store, sink| terminal.run(store, sink)).await)
        }

        fn seen(&mut self) -> Vec<(u64, &'static str)> {
            self.0.seen(|screen| match screen {
                Screen::Update(update) if update.replaced => "replaced",
                Screen::Update(_) => "patched",
                Screen::Failed(_) => "failed",
                Screen::Closed => "closed",
                Screen::Fit(Unfitted::TakenBack) => "taken back",
                Screen::Fit(Unfitted::Refused(_)) => "refused",
            })
        }

        fn surfaces(&mut self) -> Vec<String> {
            self.0.launched(&["surface", "view "])
        }
    }

    #[tokio::test(start_paused = true)]
    async fn opens_the_same_surface_again_when_it_ends_with_the_tab_still_there() {
        let surfaces = vec![vec![drawn(), Said::After(5), exits()], vec![drawn()]];
        let mut watched = OnScreen::after_a_minute(watching(vec![vec![session(BOOT, 7, true)]], surfaces)).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (6, "replaced")]);
        assert_eq!(watched.surfaces(), [TAB.join(" "), TAB.join(" ")]);
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_tab_closed_and_stops_when_it_is_gone_from_the_session() {
        let sessions = vec![vec![session(BOOT, 7, true), Said::After(5), session(BOOT, 8, false)]];
        let surfaces = vec![vec![drawn(), Said::After(5), exits()], vec![drawn()]];
        let mut watched = OnScreen::after_a_minute(watching(sessions, surfaces)).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (6, "closed")]);
        assert_eq!((watched.surfaces().len(), watched.0.running.is_finished()), (1, true));
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_tab_closed_without_opening_it_when_the_session_lacks_it() {
        let mut watched = OnScreen::after_a_minute(watching(vec![vec![session(BOOT, 7, false)]], Vec::new())).await;
        assert_eq!((watched.seen(), watched.surfaces()), (vec![(0, "closed")], Vec::new()));
    }

    #[tokio::test(start_paused = true)]
    async fn opens_it_again_once_connected_after_the_store_lost_the_transport_with_the_surface_still_open() {
        let sessions =
            vec![vec![session(BOOT, 7, true), Said::After(5), exits()], vec![Said::After(20), session(BOOT, 7, true)]];
        let mut watched = OnScreen::after_a_minute(watching(sessions, vec![vec![drawn()]])).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (26, "replaced")]);
        assert_eq!(watched.surfaces().len(), 2);
    }

    fn refused() -> Said {
        fake::refused("no tab 3")
    }

    #[tokio::test(start_paused = true)]
    async fn tells_once_of_a_surface_that_keeps_failing_with_the_tab_still_there() {
        let mut watched = OnScreen::after_a_minute(watching(connected(), vec![vec![refused()]])).await;
        assert_eq!(watched.seen(), [(0, "failed")]);
        assert!(watched.surfaces().len() > 50);
    }

    #[tokio::test(start_paused = true)]
    async fn tells_again_of_a_failure_after_the_surface_showed() {
        let surfaces = vec![vec![refused()], vec![drawn(), Said::After(5), refused()], vec![drawn()]];
        let mut watched = OnScreen::after_a_minute(watching(connected(), surfaces)).await;
        assert_eq!(watched.seen(), [(0, "failed"), (1, "replaced"), (6, "failed"), (7, "replaced")]);
    }

    #[tokio::test(start_paused = true)]
    async fn tells_of_a_surface_lost_while_the_store_kept_its_transport() {
        let surfaces = vec![vec![drawn(), Said::After(5), dropped()], vec![drawn()]];
        let mut watched = OnScreen::after_a_minute(watching(connected(), surfaces)).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (5, "failed"), (6, "replaced")]);
    }

    #[tokio::test(start_paused = true)]
    async fn stays_quiet_about_a_surface_lost_as_the_store_drops_its_transport() {
        let surfaces = vec![vec![drawn(), Said::After(5), dropped()], vec![drawn()]];
        let mut watched = OnScreen::after_a_minute(watching(dropped_for_a_while(), surfaces)).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (26, "replaced")]);
    }

    #[tokio::test(start_paused = true)]
    async fn tells_of_a_line_that_is_no_surface_and_goes_on() {
        let surfaces = vec![vec![Said::Line("{\"boot\":".to_owned()), drawn()]];
        let mut watched = OnScreen::after_a_minute(watching(connected(), surfaces)).await;
        assert_eq!(watched.seen(), [(0, "failed"), (0, "replaced")]);
    }

    #[tokio::test(start_paused = true)]
    async fn waits_out_a_restart_and_starts_over_from_the_new_boot_s_first_frame() {
        let mut rebooted: Value = serde_json::from_str(FRAME).expect("the frame fixture");
        rebooted["boot"] = "another-boot".into();
        let surfaces = vec![vec![drawn(), Said::After(5), exits()], vec![Said::Line(rebooted.to_string())]];
        let mut watched = OnScreen::after_a_minute(Host::restarting(&TAB, surfaces)).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (15, "replaced")]);
    }

    fn viewed(tab: u64, controls: bool) -> Said {
        let mut line: Value = serde_json::from_str(VIEW).expect("the view fixture");
        line["view"]["shown"]["tab"] = tab.into();
        line["view"]["controls"] = controls.into();
        Said::Line(line.to_string())
    }

    fn drawn_for_the_view(tab: u64) -> Said {
        let mut frame: Value = serde_json::from_str(FRAME).expect("the frame fixture");
        frame["view"] = 41.into();
        frame["tab"] = tab.into();
        frame["projection"] = 3.into();
        Said::Line(frame.to_string())
    }

    fn fitting(views: Vec<Vec<Said>>, fitted: Vec<Vec<Said>>) -> Host {
        watching(connected(), vec![vec![drawn()]]).also(&CLAIM, views).also(&VIEW_ARGV, fitted)
    }

    fn granted() -> Vec<Vec<Said>> {
        vec![vec![viewed(3, true)]]
    }

    #[tokio::test(start_paused = true)]
    async fn claims_the_tab_with_a_view_and_shows_the_surface_of_the_view() {
        let mut watched = OnScreen::fitted_for_a_minute(fitting(granted(), vec![vec![drawn_for_the_view(3)]])).await;
        assert_eq!(watched.seen(), [(0, "replaced")]);
        assert_eq!(watched.surfaces(), [CLAIM.join(" "), VIEW_ARGV.join(" ")]);
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_view_was_refused_and_shows_the_tab_as_it_is_from_then_on() {
        let mut watched = OnScreen::fitted_for_a_minute(fitting(vec![vec![refused()]], Vec::new())).await;
        assert_eq!(watched.seen(), [(0, "refused"), (0, "replaced")]);
        assert_eq!(watched.surfaces(), [CLAIM.join(" "), TAB.join(" ")]);
    }

    #[tokio::test(start_paused = true)]
    async fn shows_the_tab_as_it_is_once_the_machine_takes_it_back() {
        let views = vec![vec![viewed(3, true), Said::After(5), viewed(3, false)]];
        let mut watched = OnScreen::fitted_for_a_minute(fitting(views, vec![vec![drawn_for_the_view(3)]])).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (5, "taken back"), (5, "replaced")]);
        assert_eq!(watched.surfaces(), [CLAIM.join(" "), VIEW_ARGV.join(" "), TAB.join(" ")]);
    }

    #[tokio::test(start_paused = true)]
    async fn paints_nothing_of_the_tab_the_view_moved_to_and_says_the_tab_closed() {
        let sessions = vec![vec![session(BOOT, 7, true), Said::After(5), session(BOOT, 8, false)]];
        let views = vec![vec![viewed(3, true), Said::After(5), viewed(9, true)]];
        let fitted = vec![vec![drawn_for_the_view(3), Said::After(5), drawn_for_the_view(9)]];
        let mut watched = OnScreen::fitted_for_a_minute(Host { sessions, ..fitting(views, fitted) }).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (6, "closed")]);
        assert_eq!(watched.surfaces(), [CLAIM.join(" "), VIEW_ARGV.join(" ")]);
        assert!(watched.0.running.is_finished());
    }

    #[tokio::test(start_paused = true)]
    async fn claims_the_tab_again_at_the_same_size_once_connected_again() {
        let sessions =
            vec![vec![session(BOOT, 7, true), Said::After(5), exits()], vec![Said::After(20), session(BOOT, 7, true)]];
        let host = Host { sessions, ..fitting(granted(), vec![vec![drawn_for_the_view(3)]]) };
        let mut watched = OnScreen::fitted_for_a_minute(host).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (26, "replaced")]);
        let claimed = [CLAIM.join(" "), VIEW_ARGV.join(" ")];
        assert_eq!(watched.surfaces(), [claimed.clone(), claimed].concat());
    }

    #[tokio::test(start_paused = true)]
    async fn stays_quiet_about_a_view_lost_as_the_store_drops_its_transport() {
        let sessions = vec![
            vec![session(BOOT, 7, true), Said::After(5), dropped()],
            vec![Said::After(20), session(BOOT, 7, true)],
        ];
        let views = vec![vec![viewed(3, true), Said::After(5), dropped()], granted().concat()];
        let host = Host { sessions, ..fitting(views, vec![vec![drawn_for_the_view(3)]]) };
        let mut watched = OnScreen::fitted_for_a_minute(host).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (26, "replaced")]);
        assert_eq!(watched.surfaces().len(), 4);
    }

    #[tokio::test(start_paused = true)]
    async fn waits_for_the_session_to_drop_the_tab_the_view_moved_away_from() {
        let sessions = vec![vec![session(BOOT, 7, true), Said::After(7), session(BOOT, 8, false)]];
        let views = vec![vec![viewed(3, true), Said::After(5), viewed(9, true)], vec![refused()]];
        let fitted = vec![vec![drawn_for_the_view(3)]];
        let mut watched = OnScreen::fitted_for_a_minute(Host { sessions, ..fitting(views, fitted) }).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (8, "closed")]);
        assert_eq!(watched.surfaces(), [CLAIM.join(" "), VIEW_ARGV.join(" ")]);
    }

    #[tokio::test(start_paused = true)]
    async fn claims_the_tab_again_when_the_session_keeps_it_after_the_view_moved_away() {
        let views = vec![vec![viewed(3, true), Said::After(5), viewed(9, true)], granted().concat()];
        let mut watched = OnScreen::fitted_for_a_minute(fitting(views, vec![vec![drawn_for_the_view(3)]])).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (11, "replaced")]);
        assert_eq!(watched.surfaces().len(), 4);
    }

    #[tokio::test(start_paused = true)]
    async fn stays_quiet_about_a_view_refused_as_the_store_drops_its_transport() {
        let sessions = vec![vec![session(BOOT, 7, true), Said::After(5), dropped()]];
        let views = vec![vec![Said::After(5), refused()], granted().concat()];
        let host = Host { sessions, ..fitting(views, vec![vec![drawn_for_the_view(3)]]) };
        let mut watched = OnScreen::fitted_for_a_minute(host).await;
        assert!(!watched.seen().contains(&(5, "refused")));
    }

    fn stale() -> Said {
        Said::Ends(Ok(Exit { code: 4, stderr: "boot changed".to_owned() }))
    }

    #[tokio::test(start_paused = true)]
    async fn stays_fitted_through_a_restart_told_by_the_view_before_the_store() {
        let views = vec![vec![viewed(3, true), Said::After(3), stale()], Vec::new(), granted().concat()];
        let restarting = Host::restarting(&TAB, vec![vec![drawn()]]);
        let host = restarting.also(&CLAIM, views).also(&VIEW_ARGV, vec![vec![drawn_for_the_view(3)]]);
        let mut watched = OnScreen::fitted_for_a_minute(host).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (3, "failed"), (15, "replaced")]);
        let claims = watched.surfaces().into_iter().filter(|argv| *argv == CLAIM.join(" ")).count();
        assert_eq!(claims, 3);
    }

    #[tokio::test(start_paused = true)]
    async fn stays_fitted_when_the_view_is_lost_while_the_store_keeps_its_transport() {
        let views = vec![vec![viewed(3, true), Said::After(5), dropped()], granted().concat()];
        let mut watched = OnScreen::fitted_for_a_minute(fitting(views, vec![vec![drawn_for_the_view(3)]])).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (5, "failed"), (6, "replaced")]);
        let claimed = [CLAIM.join(" "), VIEW_ARGV.join(" ")];
        assert_eq!(watched.surfaces(), [claimed.clone(), claimed].concat());
    }

    #[tokio::test(start_paused = true)]
    async fn claims_the_tab_again_when_the_surface_of_the_view_ends() {
        let fitted = vec![vec![drawn_for_the_view(3), Said::After(5), exits()], vec![drawn_for_the_view(3)]];
        let mut watched = OnScreen::fitted_for_a_minute(fitting(granted(), fitted)).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (6, "replaced")]);
        assert_eq!(watched.surfaces().len(), 4);
    }
}
