use cornercase_core::cli::surface::{Modes, MouseMode, Style};
use cornercase_core::error::Error;
use cornercase_core::terminal::grid::{Line, Run};
use rstest::rstest;
use serde_json::{Value, json};
use tokio::sync::mpsc::UnboundedSender;

use super::*;
use crate::commands::connection::start;
use crate::commands::error::ErrorCode;
use crate::commands::machines::remove;
use crate::test_util::{BOOT, Ipc, Recorder, Streams, Studio, connected_to, launcher, serving, settle, spawned};

const SURFACE: [&str; 8] = ["surface", "--tab", "3", "--cols", "80", "--rows", "24", "--patches"];
const CLAIM: [&str; 7] = ["view", "--tab", "3", "--cols", "80", "--rows", "24"];
const VIEWED: [&str; 4] = ["surface", "--view", "41", "--patches"];
const TAB: Terminal = Terminal { tab: 3, pane: None, size: Size { cols: 80, rows: 24 }, fit: false };
const FITTED: Terminal = Terminal { fit: true, ..TAB };

fn line(text: &str) -> Line {
    let width = u16::try_from(text.len()).expect("a short text");
    vec![Run { text: text.to_owned(), width, style: Style::default(), link: None }]
}

fn shown(pane: u64, replaced: bool, rows: &[(u16, &str)]) -> Update {
    let rows = rows.iter().map(|(row, text)| (*row, line(text))).collect();
    let size = Size { cols: 80, rows: 24 };
    Update { pane, size, cursor: None, colours: Colours::default(), modes: Modes::default(), replaced, rows }
}

fn update(pane: u64, replaced: bool, rows: &[(u16, &str)]) -> Screen {
    Screen::Update(shown(pane, replaced, rows))
}

fn moded(modes: Modes) -> Screen {
    Screen::Update(Update { modes, ..shown(1, false, &[]) })
}

fn texts(changed: &TerminalChanged) -> Vec<(u16, String)> {
    changed.rows.iter().map(|(row, runs)| (*row, runs.iter().map(|run| run.text.as_str()).collect())).collect()
}

fn owned(rows: &[(u16, &str)]) -> Vec<(u16, String)> {
    rows.iter().map(|(row, text)| (*row, (*text).to_owned())).collect()
}

fn batched(screens: Vec<Screen>) -> TerminalChanged {
    let mut batch = Batch::new(MachineId(7), 3);
    for screen in screens {
        batch.add(screen);
    }
    batch.pending.expect("something to emit")
}

fn with_tabs(tabs: &[u64]) -> String {
    let tabs: Vec<Value> = tabs.iter().map(|id| json!({ "id": id })).collect();
    json!({ "boot": BOOT, "revision": 7, "version": "first", "projects": [{ "workspaces": [{ "tabs": tabs }] }] })
        .to_string()
}

fn drawn() -> String {
    let lines = json!([[{ "cells": ["a"] }], [{ "cells": ["b"] }]]);
    let pane = json!({ "pane": 1, "active": true, "size": { "cols": 80, "rows": 2 }, "lines": lines });
    json!({ "boot": BOOT, "surface": 1, "revision": 1, "tab": 3, "panes": [pane] }).to_string()
}

fn granted() -> String {
    let mut line: Value = serde_json::from_str(&with_tabs(&[3])).expect("a session");
    line["view"] = json!({ "id": 41, "projection": 1, "active": true, "cols": 80, "rows": 24,
        "shown": { "tab": 3 }, "controls": true });
    line.to_string()
}

fn drawn_for_the_view() -> String {
    let mut frame: Value = serde_json::from_str(&drawn()).expect("a frame");
    frame["view"] = 41.into();
    frame["projection"] = 1.into();
    frame.to_string()
}

struct Opened {
    studio: Studio,
    surfaces: Streams,
    views: Streams,
    recorder: Recorder,
}

impl Opened {
    async fn with_tabs(tabs: &[u64]) -> Self {
        Self::showing(tabs, TAB).await
    }

    async fn showing(tabs: &[u64], terminal: Terminal) -> Self {
        let (surfaces, views) = (Streams::default(), Streams::default());
        let fake = surfaces.on(serving(&[with_tabs(tabs)]), &SURFACE, &[&drawn()]);
        let studio = connected_to(views.on(views.on(fake, &CLAIM, &[&granted()]), &VIEWED, &[&drawn_for_the_view()]));
        let recorder = Recorder::default();
        open(&studio.state, recorder.clone(), spawned, terminal).expect("the terminal opens");
        settle().await;
        Self { studio, surfaces, views, recorder }
    }

    fn running(&self) -> (usize, usize) {
        (self.views.running(), self.surfaces.running())
    }
}

mod a_batch {
    use super::*;

    #[test]
    fn joins_the_rows_of_its_updates_by_number_with_the_latest_winning() {
        let changed = batched(vec![update(1, false, &[(0, "a"), (1, "b")]), update(1, false, &[(1, "c"), (2, "d")])]);
        assert_eq!((changed.replaced, texts(&changed)), (false, owned(&[(0, "a"), (1, "c"), (2, "d")])));
    }

    #[test]
    fn drops_the_rows_before_an_update_that_replaced_them() {
        let changed = batched(vec![update(1, false, &[(0, "a"), (5, "x")]), update(2, true, &[(0, "b")])]);
        assert_eq!((changed.pane, changed.replaced, texts(&changed)), (Some(2), true, owned(&[(0, "b")])));
    }

    #[test]
    fn stays_replaced_after_an_update_that_was_not() {
        let changed = batched(vec![update(1, true, &[(0, "a")]), update(1, false, &[(1, "b")])]);
        assert_eq!((changed.replaced, texts(&changed)), (true, owned(&[(0, "a"), (1, "b")])));
    }

    #[test]
    fn closes_with_the_rows_it_held() {
        let changed = batched(vec![update(1, false, &[(0, "a")]), Screen::Closed]);
        assert_eq!((changed.closed, changed.pane, texts(&changed)), (true, Some(1), owned(&[(0, "a")])));
    }

    #[test]
    fn closes_without_a_pane_when_it_showed_nothing() {
        let changed = batched(vec![Screen::Closed]);
        assert_eq!((changed.machine, changed.tab, changed.closed, changed.pane), (7, 3, true, None));
        assert_eq!(changed.modes, Modes::default());
    }

    #[test]
    fn keeps_the_modes_of_its_latest_update() {
        let mouse = Modes { mouse: MouseMode::PressRelease, ..Modes::default() };
        let alternate = Modes { alternate_screen: true, alternate_scroll: true, ..Modes::default() };
        assert_eq!(batched(vec![moded(mouse), moded(alternate)]).modes, alternate);
    }

    #[test]
    fn tells_a_failure_as_a_toast_after_the_rows_it_held_and_only_once() {
        let failed = Error::Failed { code: 1, stderr: "no tab 3".to_owned() };
        let mut batch = Batch::new(MachineId(7), 3);
        batch.add(update(1, false, &[(0, "a")]));
        batch.add(Screen::Failed(failed.clone()));
        let taken = batch.take();
        assert!(matches!(&taken[..], [Emitted::Terminal(_), _]), "{taken:?}");
        assert_eq!(taken[1], warned(&failed));
        assert_eq!(batch.take(), []);
    }

    #[rstest]
    #[case::when_the_machine_takes_the_tab_back(Unfitted::TakenBack, Toast::info("The machine took this tab back"))]
    #[case::when_the_view_is_refused(
        Unfitted::Refused(Error::Failed { code: 2, stderr: "view 41 cannot size a tab now".to_owned() }),
        Toast::error("cornercase exited with code 2: view 41 cannot size a tab now")
    )]
    fn turns_the_fit_off_and_tells_why(#[case] unfitted: Unfitted, #[case] toast: Toast) {
        let mut batch = Batch::new(MachineId(7), 3);
        batch.add(Screen::Fit(unfitted));
        let taken = batch.take();
        let [Emitted::Terminal(changed), told] = &taken[..] else { panic!("a change and a toast: {taken:?}") };
        assert_eq!((changed.unfitted, changed.closed, changed.pane), (true, false, None));
        assert_eq!(*told, Emitted::Toast(toast));
    }
}

mod by_frames {
    use super::*;

    async fn after(millis: u64, screens: &UnboundedSender<Screen>, screen: Screen) {
        time::sleep(Duration::from_millis(millis)).await;
        screens.send(screen).expect("the frames are still read");
    }

    #[tokio::test(start_paused = true)]
    async fn emits_the_first_update_at_once_and_joins_the_next_ones_until_the_frame_ends() {
        let recorder = Recorder::default();
        let (screens, heard) = mpsc::unbounded_channel();
        let emitting = tokio::spawn(by_frame(Batch::new(MachineId(7), 3), heard, recorder.clone()));
        after(0, &screens, update(1, true, &[(0, "a")])).await;
        after(2, &screens, update(1, false, &[(1, "b")])).await;
        after(2, &screens, update(1, false, &[(2, "c")])).await;
        assert_eq!(recorder.terminals().len(), 1);
        time::sleep(Duration::from_millis(13)).await;
        let emitted: Vec<_> = recorder.terminals().iter().map(texts).collect();
        assert_eq!(emitted, [owned(&[(0, "a")]), owned(&[(1, "b"), (2, "c")])]);
        drop(screens);
        emitting.await.expect("it ends once the screens end");
    }
}

mod opening {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn needs_a_connected_machine() {
        let studio = Studio::new();
        let refused = open(&studio.state, Recorder::default(), spawned, TAB).expect_err("no machine");
        assert_eq!(refused.code, ErrorCode::NotConnected);
    }

    #[tokio::test(start_paused = true)]
    async fn shows_the_surface_of_the_tab() {
        let opened = Opened::with_tabs(&[3]).await;
        let shown = opened.recorder.terminals();
        let states: Vec<_> = shown.iter().map(|it| (it.machine, it.pane, it.replaced, it.closed)).collect();
        assert_eq!(states, [(opened.studio.id.0, Some(1), true, false)]);
        assert_eq!(texts(&shown[0]), owned(&[(0, "a"), (1, "b")]));
        assert_eq!(opened.surfaces.running(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_tab_closed_when_the_session_lacks_it() {
        let opened = Opened::with_tabs(&[4]).await;
        let closed: Vec<_> = opened.recorder.terminals().iter().map(|it| (it.closed, it.pane)).collect();
        assert_eq!(closed, [(true, None)]);
        assert_eq!(opened.surfaces.running(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn again_stops_the_surface_it_showed_before() {
        let opened = Opened::with_tabs(&[3]).await;
        open(&opened.studio.state, Recorder::default(), spawned, TAB).expect("the terminal opens again");
        settle().await;
        assert_eq!(opened.surfaces.running(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn fitted_claims_the_tab_with_a_view_and_shows_its_surface() {
        let opened = Opened::showing(&[3], FITTED).await;
        assert_eq!(opened.running(), (2, 0));
        let shown: Vec<_> = opened.recorder.terminals().iter().map(|it| (it.pane, it.unfitted)).collect();
        assert_eq!(shown, [(Some(1), false)]);
    }

    #[tokio::test(start_paused = true)]
    async fn again_without_the_fit_lets_the_view_go_and_shows_the_tab_as_it_is() {
        let opened = Opened::showing(&[3], FITTED).await;
        open(&opened.studio.state, Recorder::default(), spawned, TAB).expect("the terminal opens again");
        settle().await;
        assert_eq!(opened.running(), (0, 1));
    }
}

mod stopping {
    use super::*;

    fn closing(studio: &Studio) {
        close(&studio.state, TAB.tab);
    }

    fn connecting_again(studio: &Studio) {
        let (launcher, _) = launcher(serving(&[with_tabs(&[3])]));
        start(&studio.state, Recorder::default(), &launcher, studio.id).expect("connected again");
    }

    fn removing_the_machine(studio: &Studio) {
        remove(&studio.state, &Recorder::default(), studio.id).expect("the machine is removed");
    }

    #[rstest]
    #[case::when_it_closes(closing)]
    #[case::when_a_machine_connects_again(connecting_again)]
    #[case::when_its_machine_is_removed(removing_the_machine)]
    #[tokio::test(start_paused = true)]
    async fn cuts_the_surface(#[case] stop: fn(&Studio)) {
        let opened = Opened::with_tabs(&[3]).await;
        stop(&opened.studio);
        settle().await;
        assert_eq!(opened.surfaces.running(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn leaves_the_surface_of_another_tab_when_closing() {
        let opened = Opened::with_tabs(&[3]).await;
        close(&opened.studio.state, 4);
        settle().await;
        assert_eq!(opened.surfaces.running(), 1);
    }
}

mod over_ipc {
    use super::*;

    #[test]
    fn refuses_to_open_without_a_connected_machine() {
        let ipc = Ipc::new(Studio::new().reloaded());
        let shape = json!({ "size": { "cols": 80, "rows": 24 }, "fit": true });
        let refused = ipc.invoke("terminal_open", json!({ "tab": 3, "pane": null, "shape": shape }));
        assert_eq!(refused.expect_err("no machine")["code"], "not_connected");
    }

    #[test]
    fn closes_nothing_without_a_terminal() {
        let ipc = Ipc::new(Studio::new().reloaded());
        assert_eq!(ipc.invoke("terminal_close", json!({ "tab": 3 })), Ok(Value::Null));
    }

    #[tokio::test(start_paused = true)]
    async fn cuts_the_surface_it_showed_when_closing() {
        let studio = Studio::new();
        let state = studio.reloaded();
        let surfaces = Streams::default();
        let (launcher, _) = launcher(surfaces.on(serving(&[with_tabs(&[3])]), &SURFACE, &[&drawn()]));
        start(&state, Recorder::default(), &launcher, studio.id).expect("connected");
        open(&state, Recorder::default(), spawned, TAB).expect("the terminal opens");
        settle().await;
        assert_eq!(surfaces.running(), 1);
        let ipc = Ipc::new(state);
        assert_eq!(ipc.invoke("terminal_close", json!({ "tab": 3 })), Ok(Value::Null));
        settle().await;
        assert_eq!(surfaces.running(), 0);
    }
}
