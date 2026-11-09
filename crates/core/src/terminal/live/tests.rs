use std::future;
use std::time::Duration;

use futures_util::SinkExt;
use rstest::rstest;
use serde_json::Value;
use tokio::time;

use super::*;
use crate::cli::surface::fixtures::{FRAME, PATCH};
use crate::transport::fake::FakeTransport;
use crate::transport::{Out, OutSink};

const ARGV: [&str; 8] = ["surface", "--tab", "12", "--cols", "80", "--rows", "24", "--patches"];
const SIZE: Size = Size { cols: 80, rows: 24 };
const TAB_12: Source = Source::Tab { tab: 12, size: SIZE };
const VIEW_ARGV: [&str; 4] = ["surface", "--view", "41", "--patches"];
const VIEW_41: Source = Source::View { view: 41, tab: 12 };

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

#[derive(Clone)]
enum Said {
    Line(String),
    After(u64),
    Ends(End),
}

fn exits() -> Said {
    Said::Ends(Ok(Exit { code: 0, stderr: String::new() }))
}

async fn played(mut out: OutSink, said: Vec<Said>) -> End {
    for said in said {
        match said {
            Said::Line(line) => out.send(Out::Stdout(line)).await.map_err(|_| Error::OutputClosed)?,
            Said::After(seconds) => time::sleep(Duration::from_secs(seconds)).await,
            Said::Ends(end) => return end,
        }
    }
    future::pending().await
}

mod open {
    use super::*;

    #[tokio::test]
    async fn launches_only_the_surface_of_the_tab_at_its_size_with_patches() {
        let (transport, mut argv) = FakeTransport::default().answer(&ARGV, &[], &[], 0).launched();
        let live = Live::open(transport, idle_queue(), TAB_12);
        assert_eq!(steps(live).await, [Step::Ended(Ended::Exited)]);
        assert_eq!(argv.recv().await, Some(ARGV.join(" ")));
        assert!(argv.try_recv().is_err());
    }

    #[tokio::test]
    async fn launches_the_surface_of_a_view_with_patches() {
        let (transport, mut launched) = FakeTransport::default().answer(&VIEW_ARGV, &[], &[], 0).launched();
        let live = Live::open(transport, idle_queue(), VIEW_41);
        assert_eq!(steps(live).await, [Step::Ended(Ended::Exited)]);
        assert_eq!(launched.recv().await, Some(VIEW_ARGV.join(" ")));
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
    #[tokio::test]
    async fn does_not_apply_a_line_meant_for_another(#[case] json: &str, #[case] edit: fn(&mut Value)) {
        let mut live = printing(&[of_view(FRAME, |_| {}), of_view(json, edit)]);
        assert_eq!(live.next().await, whole_frame());
        assert_eq!(live.next().await, Step::Ended(Ended::Exited));
        assert_eq!((live.grid().tab, live.grid().revision), (12, 3));
    }

    #[tokio::test]
    async fn hands_nothing_of_a_frame_older_than_the_projection_it_saw() {
        let mut live = printing(&[at(FRAME, 2)]);
        live.saw_projection(3);
        assert_eq!(steps(live).await, [Step::Ended(Ended::Exited)]);
    }

    #[tokio::test]
    async fn hands_the_first_complete_frame_after_the_projection_rose_as_replaced() {
        let mut live = printing(&[at(FRAME, 2), at(&without_copy(), 2), at(FRAME, 3)]);
        assert_eq!(live.next().await, whole_frame());
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

    #[tokio::test]
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tokio::task::JoinHandle;
    use tokio::time::Instant;

    use super::*;
    use crate::cli::contract::fixtures::REAL;
    use crate::cli::status::fixtures::WATCH;
    use crate::machines::pick::PickConfig;
    use crate::machines::route::Route;
    use crate::machines::{Machine, MachineId};

    const BOOT: &str = "4242-1760600000";
    const RESTARTED: &str = "4243-1760600100";
    const TAB: [&str; 8] = ["surface", "--tab", "3", "--cols", "80", "--rows", "24", "--patches"];

    fn contract(boot: &str) -> Vec<Said> {
        let mut contract: Value = serde_json::from_str(REAL).expect("the real contract");
        contract["boot"] = boot.into();
        vec![Said::Line(contract.to_string()), exits()]
    }

    fn session(boot: &str, revision: u64, tabs: bool) -> Said {
        let mut session: Value = serde_json::from_str(WATCH).expect("the watch fixture");
        (session["boot"], session["revision"]) = (boot.into(), revision.into());
        if !tabs {
            session["projects"] = Value::Array(Vec::new());
        }
        Said::Line(session.to_string())
    }

    fn drawn() -> Said {
        Said::Line(one_line(FRAME))
    }

    fn in_turn(fake: FakeTransport, argv: &[&str], turns: Vec<Vec<Said>>) -> FakeTransport {
        let call = AtomicUsize::new(0);
        fake.script(argv, move |_, out| {
            let turn = turns.get(call.fetch_add(1, Ordering::SeqCst)).or(turns.last());
            played(out, turn.cloned().unwrap_or_default())
        })
    }

    struct Mac {
        contracts: Vec<Vec<Said>>,
        sessions: Vec<Vec<Said>>,
        surfaces: Vec<Vec<Said>>,
    }

    impl Mac {
        fn of(sessions: Vec<Vec<Said>>, surfaces: Vec<Vec<Said>>) -> Self {
            Self { contracts: vec![contract(BOOT)], sessions, surfaces }
        }

        fn fake(self) -> FakeTransport {
            let fake = FakeTransport::default().open(&["events", "--json", "--notices"], &[]);
            let fake = in_turn(fake.open(&["status", "--app", "--follow", "--json"], &[]), &TAB, self.surfaces);
            let fake = in_turn(fake, &["contract", "--json"], self.contracts);
            in_turn(fake, &["status", "--follow", "--json"], self.sessions)
        }
    }

    struct Watched {
        screens: mpsc::UnboundedReceiver<(Duration, Screen)>,
        launched: mpsc::UnboundedReceiver<String>,
        running: JoinHandle<()>,
    }

    impl Watched {
        async fn after_a_minute(mac: Mac) -> Self {
            let (transport, launched) = mac.fake().launched();
            let studio = Route::parse("ws://studio.local:8765/?token=c2VjcmV0").expect("a bridge url");
            let machine =
                Machine { id: MachineId(1), name: "studio".to_owned(), routes: vec![studio], last_route: None };
            let (store, driver) = Store::new(machine, move |_: &Route| Ok(transport.clone()), PickConfig::default());
            tokio::spawn(driver);
            let (sender, screens) = mpsc::unbounded_channel();
            let started = Instant::now();
            let terminal = Terminal { tab: 3, pane: None, size: SIZE };
            let running = tokio::spawn(terminal.run(store, move |screen| {
                sender.send((started.elapsed(), screen)).expect("the test reads every screen");
            }));
            time::sleep(Duration::from_secs(60)).await;
            Self { screens, launched, running }
        }

        fn seen(&mut self) -> Vec<(u64, &'static str)> {
            let mut seen = Vec::new();
            while let Ok((at, screen)) = self.screens.try_recv() {
                let what = match screen {
                    Screen::Update(update) if update.replaced => "replaced",
                    Screen::Update(_) => "patched",
                    Screen::Failed(_) => "failed",
                    Screen::Closed => "closed",
                };
                seen.push((at.as_secs(), what));
            }
            seen
        }

        fn surfaces(&mut self) -> Vec<String> {
            let mut surfaces = Vec::new();
            while let Ok(argv) = self.launched.try_recv() {
                surfaces.extend(argv.starts_with("surface").then_some(argv));
            }
            surfaces
        }
    }

    #[tokio::test(start_paused = true)]
    async fn opens_the_same_surface_again_when_it_ends_with_the_tab_still_there() {
        let surfaces = vec![vec![drawn(), Said::After(5), exits()], vec![drawn()]];
        let mut watched = Watched::after_a_minute(Mac::of(vec![vec![session(BOOT, 7, true)]], surfaces)).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (6, "replaced")]);
        assert_eq!(watched.surfaces(), [TAB.join(" "), TAB.join(" ")]);
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_tab_closed_and_stops_when_it_is_gone_from_the_session() {
        let sessions = vec![vec![session(BOOT, 7, true), Said::After(5), session(BOOT, 8, false)]];
        let surfaces = vec![vec![drawn(), Said::After(5), exits()], vec![drawn()]];
        let mut watched = Watched::after_a_minute(Mac::of(sessions, surfaces)).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (6, "closed")]);
        assert_eq!((watched.surfaces().len(), watched.running.is_finished()), (1, true));
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_tab_closed_without_opening_it_when_the_session_lacks_it() {
        let mut watched = Watched::after_a_minute(Mac::of(vec![vec![session(BOOT, 7, false)]], Vec::new())).await;
        assert_eq!((watched.seen(), watched.surfaces()), (vec![(0, "closed")], Vec::new()));
    }

    #[tokio::test(start_paused = true)]
    async fn opens_it_again_once_connected_after_the_store_lost_the_transport_with_the_surface_still_open() {
        let sessions =
            vec![vec![session(BOOT, 7, true), Said::After(5), exits()], vec![Said::After(20), session(BOOT, 7, true)]];
        let mut watched = Watched::after_a_minute(Mac::of(sessions, vec![vec![drawn()]])).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (26, "replaced")]);
        assert_eq!(watched.surfaces().len(), 2);
    }

    fn connected() -> Vec<Vec<Said>> {
        vec![vec![session(BOOT, 7, true)]]
    }

    fn refused() -> Said {
        Said::Ends(Ok(Exit { code: 1, stderr: "no tab 3".to_owned() }))
    }

    #[tokio::test(start_paused = true)]
    async fn tells_once_of_a_surface_that_keeps_failing_with_the_tab_still_there() {
        let mut watched = Watched::after_a_minute(Mac::of(connected(), vec![vec![refused()]])).await;
        assert_eq!(watched.seen(), [(0, "failed")]);
        assert!(watched.surfaces().len() > 50);
    }

    #[tokio::test(start_paused = true)]
    async fn tells_again_of_a_failure_after_the_surface_showed() {
        let surfaces = vec![vec![refused()], vec![drawn(), Said::After(5), refused()], vec![drawn()]];
        let mut watched = Watched::after_a_minute(Mac::of(connected(), surfaces)).await;
        assert_eq!(watched.seen(), [(0, "failed"), (1, "replaced"), (6, "failed"), (7, "replaced")]);
    }

    #[tokio::test(start_paused = true)]
    async fn tells_of_a_surface_lost_while_the_store_kept_its_transport() {
        let surfaces = vec![vec![drawn(), Said::After(5), Said::Ends(Err(Error::Dropped))], vec![drawn()]];
        let mut watched = Watched::after_a_minute(Mac::of(connected(), surfaces)).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (5, "failed"), (6, "replaced")]);
    }

    #[tokio::test(start_paused = true)]
    async fn stays_quiet_about_a_surface_lost_as_the_store_drops_its_transport() {
        let sessions = vec![
            vec![session(BOOT, 7, true), Said::After(5), Said::Ends(Err(Error::Dropped))],
            vec![Said::After(20), session(BOOT, 7, true)],
        ];
        let surfaces = vec![vec![drawn(), Said::After(5), Said::Ends(Err(Error::Dropped))], vec![drawn()]];
        let mut watched = Watched::after_a_minute(Mac::of(sessions, surfaces)).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (26, "replaced")]);
    }

    #[tokio::test(start_paused = true)]
    async fn tells_of_a_line_that_is_no_surface_and_goes_on() {
        let surfaces = vec![vec![Said::Line("{\"boot\":".to_owned()), drawn()]];
        let mut watched = Watched::after_a_minute(Mac::of(connected(), surfaces)).await;
        assert_eq!(watched.seen(), [(0, "failed"), (0, "replaced")]);
    }

    #[tokio::test(start_paused = true)]
    async fn waits_out_a_restart_and_starts_over_from_the_new_boot_s_first_frame() {
        let restart = vec![session(BOOT, 7, true), Said::After(5), session(RESTARTED, 1, true)];
        let sessions = vec![restart, vec![Said::After(10), session(RESTARTED, 1, true)]];
        let mut rebooted: Value = serde_json::from_str(FRAME).expect("the frame fixture");
        rebooted["boot"] = "another-boot".into();
        let surfaces = vec![vec![drawn(), Said::After(5), exits()], vec![Said::Line(rebooted.to_string())]];
        let mac = Mac { contracts: vec![contract(BOOT), contract(RESTARTED)], ..Mac::of(sessions, surfaces) };
        let mut watched = Watched::after_a_minute(mac).await;
        assert_eq!(watched.seen(), [(0, "replaced"), (15, "replaced")]);
    }
}
