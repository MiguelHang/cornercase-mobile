use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::transport::json_error;

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Agents {
    pub agents: Vec<AgentChoice>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct AgentChoice {
    pub agent: String,
    pub command: String,
    pub installed: bool,
}

impl Agents {
    pub fn installed(self) -> Vec<AgentChoice> {
        self.agents.into_iter().filter(|choice| choice.installed).collect()
    }
}

pub fn installed(text: &str) -> Result<Vec<AgentChoice>, Error> {
    let agents: Agents = serde_json::from_str(text).map_err(|error| json_error(&error))?;
    Ok(agents.installed())
}

#[cfg(test)]
mod tests;
