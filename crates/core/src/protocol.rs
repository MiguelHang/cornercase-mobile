use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

pub const USAGE_EXIT: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub message: String,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

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
    let wire: WireMessage = serde_json::from_str(text).map_err(|error| bad_message(&error.to_string()))?;
    decode(wire)
}

pub fn first_message(text: &str) -> Result<Vec<String>, Refusal> {
    match parse_client(text)? {
        ClientMessage::Argv(argv) => Ok(argv),
        ClientMessage::Stdin(_) | ClientMessage::StdinBase64(_) | ClientMessage::Eof => {
            Err(refused("the first message must be `argv`"))
        }
    }
}

pub fn next_message(text: &str) -> Result<Input, Refusal> {
    match parse_client(text)? {
        ClientMessage::Argv(_) => Err(refused("`argv` was already sent")),
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
            .map_err(|error| bad_message(&format!("`stdin_base64` is not base64: {error}"))),
        WireMessage::Eof(true) => Ok(ClientMessage::Eof),
        WireMessage::Eof(false) => Err(bad_message("`eof` must be true")),
    }
}

fn bad_message(reason: &str) -> Refusal {
    refused(&format!("bad message: {reason}").replace(char::is_control, " "))
}

fn refused(message: &str) -> Refusal {
    Refusal { message: message.to_owned() }
}

#[cfg(test)]
mod tests;
