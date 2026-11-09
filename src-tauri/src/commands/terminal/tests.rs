use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use cornercase_core::cli::surface::Style;
use cornercase_core::terminal::grid::{Line, Run};
use cornercase_core::transport::Transport;
use cornercase_core::transport::fake::FakeTransport;
use rstest::rstest;
use serde_json::{Value, json};
use tokio::sync::mpsc::UnboundedSender;

use super::*;
use crate::commands::connection::start;
use crate::commands::error::ErrorCode;
use crate::commands::machines::remove;
use crate::test_util::{BOOT, Ipc, Recorder, Studio, launcher, serving, settle};

const SURFACE: [&str; 8] = ["surface", "--tab", "3", "--cols", "80", "--rows", "24", "--patches"];
const TAB: Terminal = Terminal { tab: 3, pane: None, size: Size { cols: 80, rows: 24 } };

fn spawned(task: Task) -> JoinHandle<()> {
    JoinHandle::Tokio(tokio::spawn(task))
}

fn line(text: &str) -> Line {
    let width = u16::try_from(text.len()).expect("a short text");
    vec![Run { text: text.to_owned(), width, style: Style::default(), link: None }]
}

fn update(pane: u64, replaced: bool, rows: &[(u16, &str)]) -> Screen {
    let rows = rows.iter().map(|(row, text)| (*row, line(text))).collect();
    let size = Size { cols: 80, rows: 24 };
    Screen::Update(Update { pane, size, cursor: None, colours: Colours::default(), replaced, rows })
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

#[derive(Clone, Default)]
struct Surfaces(Arc<AtomicUsize>);

struct Running(Arc<AtomicUsize>);

impl Drop for Running {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Surfaces {
    fn running(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }

    fn on(&self, fake: FakeTransport) -> FakeTransport {
        let drawing = Arc::new(FakeTransport::default().open(&SURFACE, &[&drawn()]));
        let count = Arc::clone(&self.0);
        fake.script(&SURFACE, move |stdin, out| {
            count.fetch_add(1, Ordering::SeqCst);
            let running = Running(Arc::clone(&count));
            let drawing = Arc::clone(&drawing);
            async move {
                let _running = running;
                drawing.run(SURFACE.map(str::to_owned).to_vec(), stdin, out).await
            }
        })
    }
}

struct Opened {
    studio: Studio,
    surfaces: Surfaces,
    recorder: Recorder,
}

impl Opened {
    async fn with_tabs(tabs: &[u64]) -> Self {
        let studio = Studio::new();
        let surfaces = Surfaces::default();
        let (launcher, _) = launcher(surfaces.on(serving(&[with_tabs(tabs)])));
        start(&studio.state, Recorder::default(), &launcher, studio.id).expect("connected");
        let recorder = Recorder::default();
        open(&studio.state, recorder.clone(), spawned, TAB).expect("the terminal opens");
        settle().await;
        Self { studio, surfaces, recorder }
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
}

mod stopping {
    use super::*;

    fn closing(studio: &Studio) {
        close(&studio.state);
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
}

mod over_ipc {
    use super::*;

    #[test]
    fn refuses_to_open_without_a_connected_machine() {
        let ipc = Ipc::new(Studio::new().reloaded());
        let size = json!({ "cols": 80, "rows": 24 });
        let refused = ipc.invoke("terminal_open", json!({ "tab": 3, "pane": null, "size": size }));
        assert_eq!(refused.expect_err("no machine")["code"], "not_connected");
    }

    #[test]
    fn closes_nothing_without_a_terminal() {
        let ipc = Ipc::new(Studio::new().reloaded());
        assert_eq!(ipc.invoke("terminal_close", json!({})), Ok(Value::Null));
    }

    #[tokio::test(start_paused = true)]
    async fn cuts_the_surface_it_showed_when_closing() {
        let studio = Studio::new();
        let state = studio.reloaded();
        let surfaces = Surfaces::default();
        let (launcher, _) = launcher(surfaces.on(serving(&[with_tabs(&[3])])));
        start(&state, Recorder::default(), &launcher, studio.id).expect("connected");
        open(&state, Recorder::default(), spawned, TAB).expect("the terminal opens");
        settle().await;
        assert_eq!(surfaces.running(), 1);
        let ipc = Ipc::new(state);
        assert_eq!(ipc.invoke("terminal_close", json!({})), Ok(Value::Null));
        settle().await;
        assert_eq!(surfaces.running(), 0);
    }
}
