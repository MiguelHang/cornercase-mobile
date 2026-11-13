use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct PaneRef {
    pub agent: Option<String>,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub pane: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub tab: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub workspace: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct AgentClosing {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub pane: u64,
    pub agent: String,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub workspace: u64,
    #[cfg_attr(feature = "specta", specta(type = Vec<u32>))]
    pub tabs: Vec<u64>,
    pub stops: Vec<PaneRef>,
    pub keeps: Vec<PaneRef>,
    #[cfg_attr(feature = "specta", specta(type = Option<u32>))]
    pub run: Option<u64>,
    pub closed: bool,
    pub warning: Option<String>,
}

pub fn agent_args(pane: u64, boot: &str, check: bool) -> Vec<String> {
    let pane = pane.to_string();
    let head = ["close", "--agent", "--pane", &pane, "--boot", boot];
    let tail: &[&str] = if check { &["--check", "--json"] } else { &["--json"] };
    head.iter().chain(tail).map(|word| (*word).to_owned()).collect()
}

#[cfg(test)]
mod tests;
