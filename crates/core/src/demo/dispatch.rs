use std::sync::{Mutex, PoisonError};

use futures_util::future;

use super::world::World;
use crate::error::Error;
use crate::transport::{Exit, Out, OutSink, emit};

const UNKNOWN: i32 = 2;

struct Answer {
    lines: Vec<Out>,
    code: i32,
    stderr: String,
    open: bool,
}

impl Answer {
    fn printed(stdout: String) -> Self {
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
}

pub(super) async fn run(world: &Mutex<World>, argv: Vec<String>, mut out: OutSink) -> Result<Exit, Error> {
    let answer = answer(&world.lock().unwrap_or_else(PoisonError::into_inner), &argv);
    answer.sent(&mut out).await
}

fn answer(world: &World, argv: &[String]) -> Answer {
    match argv.first().map(String::as_str) {
        Some("contract") if flagged(argv, "--json") => Answer::printed(world.contract()),
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
