use std::fmt;

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

#[cfg(test)]
mod tests;
