use std::iter;

use cornercase_core::cli::input::{InputEvent, Key, Modifier};
use cornercase_core::error::Error;
use cornercase_core::transport::fake::FakeTransport;
use cornercase_core::transport::{Closed, Exit, Out, OutSink, Stdin};
use futures_util::stream::BoxStream;
use futures_util::{SinkExt, StreamExt};
use rstest::rstest;
use serde_json::{Value, json};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::*;
use crate::commands::connection::start;
use crate::commands::error::ErrorCode;
use crate::test_util::{BOOT, Ipc, Recorder, Studio, launcher, serving, settle, snapshot};

const TYPIST: [&str; 4] = ["input", "--follow", "--json", "-"];
const REQUESTS: [&str; 3] = ["requests", "--follow", "-"];
const TYPED: &str = "{}";
const ANSWERED: &str = r#"{"status":0,"text":""}"#;

struct Listened {
    studio: Studio,
    heard: UnboundedReceiver<String>,
}

impl Listened {
    async fn on(argv: &[&str], answer: &'static str) -> Self {
        let (told, heard) = mpsc::unbounded_channel();
        let fake = listening(serving(&[snapshot(BOOT, 7, "first")]), argv, answer, &told);
        let studio = Studio::new();
        let (launcher, _) = launcher(fake);
        start(&studio.state, Recorder::default(), &launcher, studio.id).expect("connected");
        settle().await;
        Self { studio, heard }
    }

    fn lines(&mut self) -> Vec<Value> {
        let lines: Vec<String> = iter::from_fn(|| self.heard.try_recv().ok()).collect();
        lines.iter().map(|line| serde_json::from_str(line).expect("a json line")).collect()
    }
}

fn listening(
    fake: FakeTransport,
    argv: &[&str],
    answer: &'static str,
    told: &UnboundedSender<String>,
) -> FakeTransport {
    let told = told.clone();
    fake.script(argv, move |stdin, out| listen(told.clone(), answer, stdin, out))
}

async fn listen(
    told: UnboundedSender<String>,
    answer: &'static str,
    mut stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> Result<Exit, Error> {
    while let Some(Stdin::Text(line)) = stdin.next().await {
        told.send(line).expect("the test reads what was heard");
        out.send(Out::Stdout(answer.to_owned())).await.map_err(|Closed| Error::OutputClosed)?;
    }
    Ok(Exit { code: 0, stderr: String::new() })
}

fn ctrl_c() -> InputEvent {
    InputEvent::Key(Key { code: "c".to_owned(), mods: vec![Modifier::Ctrl], ..Key::default() })
}

fn typing(state: &AppState) -> Result<(), CommandError> {
    input(state, 9, &[ctrl_c()])
}

fn scrolling(state: &AppState) -> Result<(), CommandError> {
    scroll(state, 9, 3)
}

mod without_a_machine {
    use super::*;

    #[rstest]
    #[case::typing(typing)]
    #[case::scrolling(scrolling)]
    fn is_refused(#[case] send: fn(&AppState) -> Result<(), CommandError>) {
        let refused = send(&Studio::new().state).expect_err("no machine");
        assert_eq!(refused.code, ErrorCode::NotConnected);
    }
}

mod typing {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn sends_each_event_to_the_typist_with_its_pane() {
        let mut listened = Listened::on(&TYPIST, TYPED).await;
        let events = [InputEvent::Text("ls".to_owned()), ctrl_c()];
        input(&listened.studio.state, 9, &events).expect("the events are queued");
        settle().await;
        let typed =
            [json!({ "text": "ls", "pane": 9 }), json!({ "key": { "code": "c", "mods": ["ctrl"] }, "pane": 9 })];
        assert_eq!(listened.lines(), typed);
    }
}

mod scrolling {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn joins_the_rows_of_a_pane_into_one_request() {
        let mut listened = Listened::on(&REQUESTS, ANSWERED).await;
        scroll(&listened.studio.state, 9, 3).expect("the scroll is queued");
        scroll(&listened.studio.state, 9, 2).expect("the scroll is queued");
        settle().await;
        assert_eq!(listened.lines(), [json!({ "args": ["scroll", "--pane", "9", "--up", "5"] })]);
    }
}

mod over_ipc {
    use super::*;

    #[rstest]
    #[case::typing("terminal_input", json!({ "pane": 9, "events": [{ "text": "ls" }] }))]
    #[case::scrolling("terminal_scroll", json!({ "pane": 9, "rowsUp": 3 }))]
    fn refuses_without_a_connected_machine(#[case] command: &str, #[case] args: Value) {
        let ipc = Ipc::new(Studio::new().reloaded());
        assert_eq!(ipc.invoke(command, args).expect_err("no machine")["code"], "not_connected");
    }
}
