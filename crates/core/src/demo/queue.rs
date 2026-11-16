use std::sync::Mutex;

use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use serde::Deserialize;
use serde_json::json;

use super::dispatch::{self, Answer};
use super::world::World;
use crate::error::Error;
use crate::transport::{Exit, Out, OutSink, Stdin, emit};

const PASTED: &str = "/Users/demo/.local/state/cornercase/pasted";
const UNNAMED: &str = "pasted.png";

#[derive(Default, Deserialize)]
struct Request {
    args: Vec<String>,
}

pub(super) async fn requests(
    world: &Mutex<World>,
    stdin: BoxStream<'static, Stdin>,
    out: OutSink,
) -> Result<Exit, Error> {
    answer_lines(stdin, out, |line| {
        let request: Request = serde_json::from_str(line).unwrap_or_default();
        dispatch::answered(world, &request.args).reply()
    })
    .await
}

pub(super) async fn input(stdin: BoxStream<'static, Stdin>, out: OutSink) -> Result<Exit, Error> {
    answer_lines(stdin, out, |_| json!({ "events": 1 }).to_string()).await
}

pub(super) async fn pasted(argv: &[String], stdin: BoxStream<'static, Stdin>) -> Answer {
    let bytes = stdin.fold(0, async |bytes, read| bytes + length(&read)).await;
    let pane = dispatch::valued(argv, "--pane").and_then(|pane| pane.parse::<u64>().ok()).unwrap_or_default();
    let name = argv.iter().find_map(|word| word.strip_prefix("--name=")).unwrap_or(UNNAMED);
    Answer::printed(json!({ "pane": pane, "path": format!("{PASTED}/{name}"), "bytes": bytes }).to_string())
}

async fn answer_lines(
    mut stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
    answer: impl Fn(&str) -> String + Send,
) -> Result<Exit, Error> {
    while let Some(read) = stdin.next().await {
        if let Stdin::Text(text) = read {
            emit(&mut out, text.lines().map(|line| Out::Stdout(answer(line))).collect()).await?;
        }
    }
    Ok(Exit { code: 0, stderr: String::new() })
}

fn length(read: &Stdin) -> usize {
    match read {
        Stdin::Text(text) => text.len(),
        Stdin::Bytes(bytes) => bytes.len(),
    }
}

#[cfg(test)]
mod tests;
