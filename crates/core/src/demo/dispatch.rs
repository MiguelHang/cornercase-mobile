use std::sync::{Mutex, PoisonError};

use futures_util::future;
use futures_util::stream::BoxStream;
use serde_json::json;

use super::queue;
use super::world::World;
use crate::error::Error;
use crate::transport::{Exit, Out, OutSink, Stdin, emit, only_stdout};

const UNKNOWN: i32 = 2;

pub(super) struct Answer {
    lines: Vec<Out>,
    code: i32,
    stderr: String,
    open: bool,
}

impl Answer {
    pub(super) fn printed(stdout: String) -> Self {
        Self { lines: vec![Out::Stdout(stdout)], code: 0, stderr: String::new(), open: false }
    }

    fn followed(stdout: Vec<String>) -> Self {
        Self { lines: stdout.into_iter().map(Out::Stdout).collect(), code: 0, stderr: String::new(), open: true }
    }

    fn unknown(argv: &[String]) -> Self {
        let stderr = format!("error: the demo has no answer for `{}`", argv.join(" "));
        Self { lines: vec![Out::Stderr(stderr.clone())], code: UNKNOWN, stderr, open: false }
    }

    async fn sent(self, out: &mut OutSink) -> Result<Exit, Error> {
        emit(out, self.lines).await?;
        if self.open {
            future::pending::<()>().await;
        }
        Ok(Exit { code: self.code, stderr: self.stderr })
    }

    pub(super) fn reply(self) -> String {
        let text: Vec<String> = self.lines.into_iter().filter_map(only_stdout).collect();
        json!({ "status": self.code, "text": text.join("\n"), "error": self.stderr }).to_string()
    }
}

pub(super) async fn run(
    world: &Mutex<World>,
    argv: Vec<String>,
    stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> Result<Exit, Error> {
    match argv.first().map(String::as_str) {
        Some("requests") if flagged(&argv, "--follow") => queue::requests(world, stdin, out).await,
        Some("input") if flagged(&argv, "--follow") && flagged(&argv, "--json") => queue::input(stdin, out).await,
        Some("paste-file") if flagged(&argv, "--json") => queue::pasted(&argv, stdin).await.sent(&mut out).await,
        _ => answered(world, &argv).sent(&mut out).await,
    }
}

pub(super) fn answered(world: &Mutex<World>, argv: &[String]) -> Answer {
    answer(&world.lock().unwrap_or_else(PoisonError::into_inner), argv)
}

fn answer(world: &World, argv: &[String]) -> Answer {
    match argv.first().map(String::as_str) {
        Some("contract") if flagged(argv, "--json") => Answer::printed(world.contract()),
        Some("send" | "keys" | "mark") if flagged(argv, "--pane") => Answer::printed(String::new()),
        Some("status") if flagged(argv, "--follow") && flagged(argv, "--json") => Answer::followed(status(world, argv)),
        Some("events") if flagged(argv, "--json") => Answer::followed(world.events()),
        _ => Answer::unknown(argv),
    }
}

fn status(world: &World, argv: &[String]) -> Vec<String> {
    vec![if flagged(argv, "--app") { world.app() } else { world.status() }]
}

fn flagged(argv: &[String], flag: &str) -> bool {
    argv.iter().skip(1).any(|word| word == flag)
}

pub(super) fn valued<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
    argv.iter().skip_while(|word| *word != flag).nth(1).map(String::as_str)
}
