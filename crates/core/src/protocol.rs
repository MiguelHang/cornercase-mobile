use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

pub const USAGE_EXIT: i32 = 2;
pub const LONGEST_LINE: usize = 16 << 20;
pub const LINE_TOO_LONG: &str = "a line of output passed 16 MiB";
pub const LARGEST_MESSAGE: usize = 1 << 20;
pub const APP_ROUTES: &[&str] = &["config", "pair", "app", "--routes", "--json"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRoutes {
    pub routes: Vec<AppRoute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRoute {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerMessage {
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

#[derive(Serialize)]
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

#[cfg(test)]
mod tests;
