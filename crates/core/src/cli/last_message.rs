use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct LastMessage {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub pane: u64,
    pub agent: String,
    pub text: String,
    pub written: Option<String>,
    pub turn_over: Option<bool>,
}

pub fn args(pane: u64) -> Vec<String> {
    ["read", "--last-message", "--pane", &pane.to_string(), "--json"].map(str::to_owned).into()
}

pub fn send_args(pane: u64) -> Vec<String> {
    ["send", "--pane", &pane.to_string(), "--enter", "--timeout", "30", "-"].map(str::to_owned).into()
}

pub fn forced_send_args(pane: u64) -> Vec<String> {
    ["send", "--pane", &pane.to_string(), "--force", "--enter", "--timeout", "30", "-"].map(str::to_owned).into()
}

#[cfg(test)]
mod tests;
