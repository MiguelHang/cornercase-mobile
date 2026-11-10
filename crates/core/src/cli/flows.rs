use serde::{Deserialize, Serialize};

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct FlowRole {
    pub role: String,
    pub agent: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct FlowChoice {
    pub name: String,
    pub description: String,
    pub roles: Vec<FlowRole>,
    pub problems: Vec<String>,
    pub default: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Flows {
    pub flows: Vec<FlowChoice>,
}

pub fn list_args(project: u64) -> Vec<String> {
    ["flow", "list", "--project", &project.to_string(), "--json"].map(str::to_owned).into()
}

#[cfg(test)]
mod tests;
