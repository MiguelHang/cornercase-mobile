use cornercase_core::cli::surface::Style;
use cornercase_core::error::Error;
use cornercase_core::terminal::grid::Run;
use cornercase_core::terminal::question::Choice;
use serde_json::{Value, json};

use super::*;
use crate::commands::error::ErrorCode;
use crate::commands::terminal;
use crate::test_util::{BOOT, Ipc, Recorder, Streams, Studio, connected_to, serving, settle, spawned};

const ASK_ONE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/surface/v1/question/ask_one.json"
));
const SURFACE: [&str; 8] = ["surface", "--tab", "942", "--cols", "80", "--rows", "24", "--patches"];
const ASKING: Asking = Asking { tab: 942, pane: 941, size: SIZE };

fn question(label: &str) -> Asked {
    let rows = vec![vec![Run { text: label.to_owned(), width: 3, style: Style::default(), link: None }]];
    let options = vec![Choice { number: 1, label: label.to_owned() }, Choice { number: 2, label: "No".to_owned() }];
    Asked::Question(Question { pane: 941, colours: Colours::default(), rows, options })
}

fn batched(said: Vec<Asked>) -> Vec<Emitted> {
    let mut asks = Asks::new(MachineId(7), ASKING);
    for asked in said {
        asks.add(asked);
    }
    asks.take()
}

fn told(said: Vec<Asked>) -> QuestionChanged {
    let taken = batched(said);
    let [Emitted::Question(changed)] = &taken[..] else { panic!("one question: {taken:?}") };
    changed.clone()
}

fn labels(changed: &QuestionChanged) -> Vec<&str> {
    changed.options.iter().map(|choice| choice.label.as_str()).collect()
}

fn with_tab(tab: u64) -> String {
    json!({ "boot": BOOT, "revision": 7, "version": "first", "projects": [{ "workspaces": [{ "tabs": [{ "id": tab }] }] }] })
        .to_string()
}

struct Watched {
    studio: Studio,
    surfaces: Streams,
    recorder: Recorder,
}

impl Watched {
    async fn in_tab(tab: u64) -> Self {
        let surfaces = Streams::default();
        let studio = connected_to(surfaces.on(serving(&[with_tab(tab)]), &SURFACE, &[ASK_ONE.trim_end()]));
        let recorder = Recorder::default();
        open(&studio.state, recorder.clone(), spawned, ASKING).expect("the question opens");
        settle().await;
        Self { studio, surfaces, recorder }
    }
}

mod asks {
    use super::*;

    #[test]
    fn join_their_questions_into_one_with_the_latest_winning() {
        let changed = told(vec![question("Red"), question("Green")]);
        assert_eq!((changed.machine, changed.tab, changed.pane, changed.closed), (7, 942, 941, false));
        assert_eq!(labels(&changed), ["Green", "No"]);
    }

    #[test]
    fn clear_the_question_without_a_toast_when_the_surface_fails() {
        let failed = Error::Failed { code: 1, stderr: "no tab 942".to_owned() };
        let changed = told(vec![question("Red"), Asked::Failed(failed)]);
        assert_eq!((changed.rows.len(), changed.options.len(), changed.closed), (0, 0, false));
    }

    #[test]
    fn say_the_tab_closed_without_a_question() {
        let changed = told(vec![question("Red"), Asked::Closed]);
        assert_eq!((changed.pane, changed.rows.len(), changed.options.len(), changed.closed), (941, 0, 0, true));
    }

    #[test]
    fn are_taken_only_once() {
        let mut asks = Asks::new(MachineId(7), ASKING);
        asks.add(question("Red"));
        asks.take();
        assert_eq!(asks.take(), []);
    }
}

mod opening {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn needs_a_connected_machine() {
        let studio = Studio::new();
        let refused = open(&studio.state, Recorder::default(), spawned, ASKING).expect_err("no machine");
        assert_eq!(refused.code, ErrorCode::NotConnected);
    }

    #[tokio::test(start_paused = true)]
    async fn reads_the_question_of_the_pane_with_its_options_from_a_passive_surface() {
        let asked = Watched::in_tab(942).await;
        let told = asked.recorder.questions();
        let [changed] = &told[..] else { panic!("one question: {told:?}") };
        assert_eq!((changed.machine, changed.pane, changed.closed), (asked.studio.id.0, 941, false));
        assert_eq!(labels(changed), ["Red", "Green", "Blue", "Type something.", "Chat about this"]);
        assert_eq!(asked.surfaces.running(), 1);
        assert_eq!(asked.recorder.toasts(), []);
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_tab_closed_when_the_session_lacks_it() {
        let asked = Watched::in_tab(4).await;
        let closed: Vec<_> = asked.recorder.questions().iter().map(|it| (it.closed, it.rows.len())).collect();
        assert_eq!(closed, [(true, 0)]);
        assert_eq!(asked.surfaces.running(), 0);
    }
}

mod closing {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn cuts_the_surface_of_its_tab() {
        let asked = Watched::in_tab(942).await;
        close(&asked.studio.state, 942);
        settle().await;
        assert_eq!(asked.surfaces.running(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn leaves_the_question_of_another_tab() {
        let asked = Watched::in_tab(942).await;
        close(&asked.studio.state, 4);
        settle().await;
        assert_eq!(asked.surfaces.running(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn the_terminal_of_the_same_tab_leaves_its_question() {
        let asked = Watched::in_tab(942).await;
        terminal::close(&asked.studio.state, 942);
        settle().await;
        assert_eq!(asked.surfaces.running(), 1);
    }
}

mod over_ipc {
    use super::*;

    #[test]
    fn refuses_to_open_without_a_connected_machine() {
        let ipc = Ipc::new(Studio::new().reloaded());
        let refused = ipc.invoke("question_open", json!({ "tab": 942, "pane": 941 }));
        assert_eq!(refused.expect_err("no machine")["code"], "not_connected");
    }

    #[tokio::test(start_paused = true)]
    async fn closes_the_surface_of_its_tab() {
        let asked = Watched::in_tab(942).await;
        let ipc = Ipc::new(asked.studio.state);
        assert_eq!(ipc.invoke("question_close", json!({ "tab": 942 })), Ok(Value::Null));
        settle().await;
        assert_eq!(asked.surfaces.running(), 0);
    }

    #[test]
    fn closes_nothing_without_a_question() {
        let ipc = Ipc::new(Studio::new().reloaded());
        assert_eq!(ipc.invoke("question_close", json!({ "tab": 942 })), Ok(Value::Null));
    }
}
