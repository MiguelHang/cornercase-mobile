use std::str::FromStr;
use std::sync::{Mutex, PoisonError};

use futures_util::stream::BoxStream;
use serde::Serialize;
use serde_json::json;

use super::follow::{self, Followed};
use super::world::World;
use super::{changes, close, empty, files, queue, start};
use crate::cli::surface::Size;
use crate::error::Error;
use crate::transport::{Exit, Out, OutSink, Stdin, emit, only_stdout};

const UNKNOWN: i32 = 2;
const FAILED: i32 = 1;

pub(super) struct Answer {
    lines: Vec<Out>,
    code: i32,
    stderr: String,
}

impl Answer {
    pub(super) fn printed(stdout: String) -> Self {
        Self { lines: vec![Out::Stdout(stdout)], code: 0, stderr: String::new() }
    }

    pub(super) fn unknown(argv: &[String]) -> Self {
        Self::refused(UNKNOWN, format!("error: the demo has no answer for `{}`", argv.join(" ")))
    }

    pub(super) fn failed(stderr: String) -> Self {
        Self::refused(FAILED, stderr)
    }

    fn refused(code: i32, stderr: String) -> Self {
        Self { lines: vec![Out::Stderr(stderr.clone())], code, stderr }
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
    Followed(Followed),
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
        Kind::Followed(followed) => follow::run(world, followed, out).await,
        Kind::Answered => answered(world, &argv).sent(&mut out).await,
    }
}

fn kind(argv: &[String]) -> Kind {
    let (follows, json) = (flagged(argv, "--follow"), flagged(argv, "--json"));
    match argv.first().map(String::as_str) {
        Some("requests") if follows => Kind::Requests,
        Some("input") if follows && json => Kind::Input,
        Some("paste-file") if json => Kind::Pasted,
        Some("status") if follows && json => Kind::Followed(Followed::Status { app: flagged(argv, "--app") }),
        Some("events") if json => Kind::Followed(Followed::Events),
        Some("view") => on_tab(argv).map_or(Kind::Answered, |(tab, size)| Kind::Followed(Followed::View { tab, size })),
        Some("surface") => surfaced(argv),
        Some("changes") if follows => listed(argv),
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
        (Some("mark"), Some(pane)) if flagged(argv, "--reviewed") => {
            world.reviewed(pane);
            Answer::printed(String::new())
        }
        (Some("mark"), Some(_)) => Answer::printed(String::new()),
        (Some("account"), _) => start::accounts(argv),
        (Some("start"), _) => start::answer(world, argv),
        (Some("close"), Some(pane)) => close::answer(world, pane, argv),
        (Some("issue" | "flow"), _) => empty::answer(argv),
        (Some("changes"), _) => changes::answer(world, argv),
        (Some("files"), _) => files::answer(world, argv),
        (Some("read"), Some(pane)) if flagged(argv, "--last-message") && flagged(argv, "--json") => world
            .last_message(pane)
            .map_or_else(|| Answer::unknown(argv), |read| read.map_or_else(Answer::failed, Answer::printed)),
        _ => Answer::unknown(argv),
    }
}

fn surfaced(argv: &[String]) -> Kind {
    if let Some(view) = number(argv, "--view") {
        return Kind::Followed(Followed::Viewed { view });
    }
    on_tab(argv).map_or(Kind::Answered, |(tab, size)| Kind::Followed(Followed::Surface { tab, size }))
}

fn listed(argv: &[String]) -> Kind {
    let mode = changes::mode(argv);
    number(argv, "--workspace")
        .map_or(Kind::Answered, |workspace| Kind::Followed(Followed::Changes { workspace, mode }))
}

fn on_tab(argv: &[String]) -> Option<(u64, Size)> {
    let size = number(argv, "--cols").zip(number(argv, "--rows")).map(|(cols, rows)| Size { cols, rows });
    number(argv, "--tab").zip(size)
}

pub(super) fn pane(argv: &[String]) -> Option<u64> {
    number(argv, "--pane")
}

pub(super) fn number<T: FromStr>(argv: &[String], flag: &str) -> Option<T> {
    valued(argv, flag)?.parse().ok()
}

pub(super) fn flagged(argv: &[String], flag: &str) -> bool {
    argv.iter().skip(1).any(|word| word == flag)
}

pub(super) fn valued<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
    argv.iter().skip_while(|word| *word != flag).nth(1).map(String::as_str)
}

pub(super) fn operands(argv: &[String]) -> impl Iterator<Item = &str> {
    argv.iter().skip_while(|word| *word != "--").skip(1).map(String::as_str)
}

pub(super) fn json(value: &impl Serialize) -> String {
    serde_json::to_string(value).expect("the demo's answers always serialize")
}
