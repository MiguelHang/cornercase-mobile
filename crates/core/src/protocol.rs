use std::fmt;
use std::net::SocketAddr;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use serde_json::error::Category;

pub const USAGE_EXIT: i32 = 2;
pub const LONGEST_LINE: usize = 16 << 20;
pub const LINE_TOO_LONG: &str = "a line of output passed 16 MiB";
pub const LARGEST_MESSAGE: usize = 1 << 20;
pub const BRIDGE_ROUTES: &str = "--bridge-routes";
pub const APP_ROUTES: &[&str] = &["config", "pair", "app", "--routes", "--json"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub message: String,
}

impl Refusal {
    pub fn new(message: &str) -> Self {
        Self { message: message.replace(char::is_control, " ") }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeRoutes {
    pub routes: Vec<BridgeRoute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeRoute {
    pub host: String,
    pub port: u16,
}

impl BridgeRoutes {
    pub fn new(addresses: &[SocketAddr]) -> Self {
        let routes =
            addresses.iter().map(|address| BridgeRoute { host: address.ip().to_string(), port: address.port() });
        Self { routes: routes.collect() }
    }

    pub fn asked(argv: &[String]) -> bool {
        matches!(argv, [only] if only == BRIDGE_ROUTES)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeMessage {
    Stdout(String),
    Stderr(String),
    Exit(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(into = "WireMessage")]
pub enum ClientMessage {
    Argv(Vec<String>),
    Stdin(String),
    StdinBase64(Vec<u8>),
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Text(String),
    Bytes(Vec<u8>),
    Eof,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireMessage {
    Argv(Vec<String>),
    Stdin(String),
    StdinBase64(String),
    Eof(bool),
}

impl From<ClientMessage> for WireMessage {
    fn from(message: ClientMessage) -> Self {
        match message {
            ClientMessage::Argv(argv) => Self::Argv(argv),
            ClientMessage::Stdin(text) => Self::Stdin(text),
            ClientMessage::StdinBase64(bytes) => Self::StdinBase64(STANDARD.encode(bytes)),
            ClientMessage::Eof => Self::Eof(true),
        }
    }
}

pub fn parse_client(text: &str) -> Result<ClientMessage, Refusal> {
    let wire: WireMessage = serde_json::from_str(text).map_err(|error| bad_message(json_reason(&error)))?;
    decode(wire)
}

pub fn first_message(text: &str) -> Result<Vec<String>, Refusal> {
    match parse_client(text)? {
        ClientMessage::Argv(argv) => Ok(argv),
        ClientMessage::Stdin(_) | ClientMessage::StdinBase64(_) | ClientMessage::Eof => {
            Err(Refusal::new("the first message must be `argv`"))
        }
    }
}

pub fn next_message(text: &str) -> Result<Input, Refusal> {
    match parse_client(text)? {
        ClientMessage::Argv(_) => Err(Refusal::new("`argv` was already sent")),
        ClientMessage::Stdin(text) => Ok(Input::Text(text)),
        ClientMessage::StdinBase64(bytes) => Ok(Input::Bytes(bytes)),
        ClientMessage::Eof => Ok(Input::Eof),
    }
}

fn decode(wire: WireMessage) -> Result<ClientMessage, Refusal> {
    match wire {
        WireMessage::Argv(argv) => Ok(ClientMessage::Argv(argv)),
        WireMessage::Stdin(text) => Ok(ClientMessage::Stdin(text)),
        WireMessage::StdinBase64(encoded) => STANDARD
            .decode(encoded)
            .map(ClientMessage::StdinBase64)
            .map_err(|_| bad_message("`stdin_base64` is not standard base64")),
        WireMessage::Eof(true) => Ok(ClientMessage::Eof),
        WireMessage::Eof(false) => Err(bad_message("`eof` must be true")),
    }
}

fn json_reason(error: &serde_json::Error) -> &'static str {
    match error.classify() {
        Category::Syntax | Category::Io => "not one JSON object with one key",
        Category::Eof => "the JSON ends too early",
        Category::Data => "not one of `argv`, `stdin`, `stdin_base64` and `eof` with a value of its type",
    }
}

fn bad_message(reason: &str) -> Refusal {
    Refusal::new(&format!("bad message: {reason}"))
}

#[cfg(test)]
mod tests;
