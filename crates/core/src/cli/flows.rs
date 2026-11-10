use serde::{Deserialize, Serialize};

use crate::cli::start::{Started, optional, other_branch, words};
use crate::error::Error;
use crate::transport::json_error;

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StartFlow {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    pub name: String,
    pub key: String,
    pub source: String,
    pub branch: String,
    pub issue_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FlowStarted {
    pub started: Started,
    pub agent: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RunInfo {
    project: u64,
    workspace: u64,
    members: Vec<Member>,
    note: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Member {
    role: String,
    agent: String,
    tab: u64,
    pane: u64,
}

const ORCHESTRATOR: &str = "orchestrator";
const NO_MEMBERS: &str = "the flow's run names no agent";

impl StartFlow {
    pub fn argv(&self) -> Vec<String> {
        let mut argv = words(&["flow", "run", "--project", &self.project.to_string()]);
        argv.push(format!("--issue={}", self.key.trim()));
        argv.extend(optional("--source", &self.source));
        argv.extend(optional("--worktree", other_branch(&self.branch, &self.issue_branch)));
        argv.extend(words(&["--json", "--", self.name.trim()]));
        argv
    }
}

pub fn flow_started(text: &str) -> Result<FlowStarted, Error> {
    let run: RunInfo = serde_json::from_str(text).map_err(|error| json_error(&error))?;
    let lead = lead(run.members).ok_or_else(|| Error::Json(NO_MEMBERS.to_owned()))?;
    let started =
        Started { project: run.project, workspace: run.workspace, tab: lead.tab, pane: lead.pane, warning: run.note };
    Ok(FlowStarted { started, agent: lead.agent })
}

fn lead(members: Vec<Member>) -> Option<Member> {
    let at = members.iter().position(|member| member.role == ORCHESTRATOR).unwrap_or(0);
    members.into_iter().nth(at)
}

pub fn list_args(project: u64) -> Vec<String> {
    ["flow", "list", "--project", &project.to_string(), "--json"].map(str::to_owned).into()
}

#[cfg(test)]
mod tests;
