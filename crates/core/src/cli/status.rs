use serde::{Deserialize, Serialize};

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Snapshot {
    pub boot: String,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub revision: u64,
    pub version: String,
    pub groups: Vec<Group>,
    pub projects: Vec<Project>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Group {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub id: u64,
    pub name: String,
    pub icon: String,
    pub colour: u8,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Project {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub id: u64,
    pub name: String,
    pub custom_name: Option<String>,
    pub path: Option<String>,
    #[cfg_attr(feature = "specta", specta(type = Option<u32>))]
    pub group: Option<u64>,
    pub workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Workspace {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub id: u64,
    pub name: String,
    pub custom_name: Option<String>,
    pub path: Option<String>,
    pub branch: Option<String>,
    pub worktree: bool,
    pub tabs: Vec<Tab>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Tab {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub id: u64,
    pub name: String,
    pub custom_name: Option<String>,
    pub status: Option<String>,
    pub active: bool,
    pub panes: Vec<Pane>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Pane {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub id: u64,
    pub agent: Option<String>,
    pub status: Option<String>,
    pub program: Option<String>,
    pub title: Option<String>,
    pub path: Option<String>,
    #[cfg_attr(feature = "specta", specta(type = Option<u32>))]
    pub parent: Option<u64>,
    pub role: Option<String>,
    pub collapsed: bool,
    pub active: bool,
    pub dialog: bool,
    pub at_prompt: Option<bool>,
}

#[cfg(test)]
mod tests;
