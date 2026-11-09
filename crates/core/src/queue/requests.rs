use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::Error;
use crate::protocol::USAGE_EXIT;
use crate::transport::{Exit, STALE_BOOT, json_error};

const NO_REQUESTS: &str = "unrecognized subcommand 'requests'";
const NO_FOLLOW: &str = "unexpected argument '--follow'";

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Answer {
    status: i32,
    json: Option<Value>,
    text: String,
    stderr: String,
    error: String,
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

pub(crate) fn absent(exit: &Exit) -> bool {
    exit.code == USAGE_EXIT && [NO_REQUESTS, NO_FOLLOW].iter().any(|refusal| exit.stderr.contains(refusal))
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
