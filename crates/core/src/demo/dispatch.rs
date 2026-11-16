use std::str::FromStr;
use std::sync::{Mutex, PoisonError};

use futures_util::stream::BoxStream;
use serde_json::json;

use super::world::World;
use super::{follow, queue};
use crate::cli::surface::Size;
use crate::error::Error;
use crate::transport::{Exit, Out, OutSink, Stdin, emit, only_stdout};

const UNKNOWN: i32 = 2;

pub(super) struct Answer {
    lines: Vec<Out>,
    code: i32,
    stderr: String,
}

impl Answer {
    pub(super) fn printed(stdout: String) -> Self {
        Self { lines: vec![Out::Stdout(stdout)], code: 0, stderr: String::new() }
    }

    fn unknown(argv: &[String]) -> Self {
        let stderr = format!("error: the demo has no answer for `{}`", argv.join(" "));
        Self { lines: vec![Out::Stderr(stderr.clone())], code: UNKNOWN, stderr }
    }

    async fn sent(self, out: &mut OutSink) -> Result<Exit, Error> {
        emit(out, self.lines).await?;
        Ok(Exit { code: self.code, stderr: self.stderr })
    }

    pub(super) fn reply(self) -> String {
        let text: Vec<String> = self.lines.into_iter().filter_map(only_stdout).collect();
        json!({ "status": self.code, "text": text.join("\n"), "error": self.stderr }).to_string()
    }
}

enum Kind {
    Requests,
    Input,
    Pasted,
    Status { app: bool },
    Events,
    Surface { tab: u64, size: Size },
    Answered,
}

pub(super) async fn run(
    world: &Mutex<World>,
    argv: Vec<String>,
    stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> Result<Exit, Error> {
    match kind(&argv) {
        Kind::Requests => queue::requests(world, stdin, out).await,
        Kind::Input => queue::input(stdin, out).await,
        Kind::Pasted => queue::pasted(&argv, stdin).await.sent(&mut out).await,
        Kind::Status { app } => follow::status(world, out, app).await,
        Kind::Events => follow::events(world, out).await,
        Kind::Surface { tab, size } => follow::surface(world, out, tab, size).await,
        Kind::Answered => answered(world, &argv).sent(&mut out).await,
    }
}

fn kind(argv: &[String]) -> Kind {
    let (follows, json) = (flagged(argv, "--follow"), flagged(argv, "--json"));
    match argv.first().map(String::as_str) {
        Some("requests") if follows => Kind::Requests,
        Some("input") if follows && json => Kind::Input,
        Some("paste-file") if json => Kind::Pasted,
        Some("status") if follows && json => Kind::Status { app: flagged(argv, "--app") },
        Some("events") if json => Kind::Events,
        Some("surface") => surfaced(argv),
        _ => Kind::Answered,
    }
}

pub(super) fn answered(world: &Mutex<World>, argv: &[String]) -> Answer {
    let mut world = world.lock().unwrap_or_else(PoisonError::into_inner);
    world.settle();
    answer(&mut world, argv)
}

fn answer(world: &mut World, argv: &[String]) -> Answer {
    match (argv.first().map(String::as_str), pane(argv)) {
        (Some("contract"), _) if flagged(argv, "--json") => Answer::printed(world.contract()),
        (Some("send"), Some(pane)) => {
            world.sent(pane);
            Answer::printed(String::new())
        }
        (Some("keys"), Some(pane)) => {
            world.pressed(pane, argv.last().map(String::as_str));
            Answer::printed(String::new())
        }
        (Some("mark"), Some(_)) => Answer::printed(String::new()),
        (Some("read"), Some(pane)) if flagged(argv, "--last-message") && flagged(argv, "--json") => {
            world.last_message(pane).map_or_else(|| Answer::unknown(argv), Answer::printed)
        }
        _ => Answer::unknown(argv),
    }
}

fn surfaced(argv: &[String]) -> Kind {
    let tab = number(argv, "--tab");
    let size = number(argv, "--cols").zip(number(argv, "--rows")).map(|(cols, rows)| Size { cols, rows });
    tab.zip(size).map_or(Kind::Answered, |(tab, size)| Kind::Surface { tab, size })
}

pub(super) fn pane(argv: &[String]) -> Option<u64> {
    number(argv, "--pane")
}

fn number<T: FromStr>(argv: &[String], flag: &str) -> Option<T> {
    valued(argv, flag)?.parse().ok()
}

fn flagged(argv: &[String], flag: &str) -> bool {
    argv.iter().skip(1).any(|word| word == flag)
}

fn valued<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
    argv.iter().skip_while(|word| *word != flag).nth(1).map(String::as_str)
}
