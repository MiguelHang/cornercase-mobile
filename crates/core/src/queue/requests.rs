use std::sync::Arc;

use futures_util::future::Either;
use serde::Deserialize;
use serde_json::{Value, json};

use super::follow::{End, Follow, Gone};
use crate::error::Error;
use crate::transport::{STALE_BOOT, Transport, json_error};

const FOLLOW: [&str; 3] = ["requests", "--follow", "-"];

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Answer {
    status: i32,
    json: Option<Value>,
    text: String,
    stderr: String,
    error: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Asked {
    Answered(Result<String, Error>),
    Unsent,
    Absent,
}

pub(crate) struct Requests {
    follow: Follow,
}

impl Requests {
    pub(crate) fn start(transport: Arc<dyn Transport>) -> Self {
        Self { follow: Follow::start(transport, &FOLLOW) }
    }

    pub(crate) async fn ask(&mut self, line: &str) -> Asked {
        match self.follow.write(line).await {
            Ok(()) => {}
            Err(None) => return Asked::Unsent,
            Err(Some(end)) => return self.unsent(&end),
        }
        match self.follow.next().await {
            Either::Left(line) => Asked::Answered(answer(&line)),
            Either::Right(end) => match self.follow.ready() {
                Some(line) => Asked::Answered(answer(&line)),
                None => self.lost(&end),
            },
        }
    }

    pub(crate) async fn ended(&mut self) {
        let _ = self.follow.end().await;
    }

    fn unsent(&self, end: &End) -> Asked {
        match self.lost(end) {
            Asked::Answered(_) => Asked::Unsent,
            asked => asked,
        }
    }

    fn lost(&self, end: &End) -> Asked {
        match self.follow.gone(end) {
            Gone::Absent => Asked::Absent,
            Gone::Unopened => Asked::Unsent,
            Gone::Lost => Asked::Answered(Err(Error::Unanswered)),
        }
    }
}

pub(crate) fn line(args: &[String], stdin: Option<&str>) -> String {
    let mut line = json!({ "args": args });
    if let Some(text) = stdin {
        line["stdin"] = Value::from(text);
    }
    line.to_string()
}

pub(crate) fn answer(line: &str) -> Result<String, Error> {
    let answer: Answer = serde_json::from_str(line).map_err(|error| json_error(&error))?;
    match answer.status {
        0 => Ok(printed(answer)),
        STALE_BOOT => Err(Error::StaleBoot(why(&answer))),
        code => Err(Error::Failed { code, stderr: why(&answer) }),
    }
}

fn printed(answer: Answer) -> String {
    let text = answer.text.strip_suffix('\n').unwrap_or(&answer.text);
    match answer.json {
        Some(json) if answer.text.is_empty() => json.to_string(),
        Some(json) => format!("{json}\n{text}"),
        None => text.to_owned(),
    }
}

fn why(answer: &Answer) -> String {
    let error = answer.error.trim_end();
    let why = if error.is_empty() { answer.stderr.trim_end() } else { error };
    why.to_owned()
}

#[cfg(test)]
mod tests;
