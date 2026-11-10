use serde::{Deserialize, Serialize};

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum SourceState {
    Connected,
    Missing,
    Failing,
    #[default]
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct SourceInfo {
    pub source: String,
    pub name: String,
    pub repository: bool,
    pub state: SourceState,
    pub missing: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct PlaceInfo {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    #[cfg_attr(feature = "specta", specta(type = Option<u32>))]
    pub workspace: Option<u64>,
    pub label: String,
    pub worktree: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Sources {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    pub sources: Vec<SourceInfo>,
    pub places: Vec<PlaceInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct IssueInfo {
    pub source: String,
    pub key: String,
    pub title: String,
    pub url: String,
    pub state: String,
    pub closed: bool,
    pub labels: Vec<String>,
    pub author: String,
    pub updated_at: String,
    pub branch: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct ListedInfo {
    pub source: String,
    pub count: u32,
    pub cached: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Lists {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    pub issues: Vec<IssueInfo>,
    pub sources: Vec<ListedInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct CommentInfo {
    pub author: String,
    pub created_at: String,
    pub body: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct ReadIssue {
    pub issue: IssueInfo,
    pub body: String,
    pub comments: Vec<CommentInfo>,
    pub prompt: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct ListFilter {
    pub source: Option<String>,
    pub closed: bool,
    pub mine: bool,
    pub cached: bool,
}

pub fn sources_args(project: u64) -> Vec<String> {
    let mut argv = command("sources", project);
    argv.push("--json".to_owned());
    argv
}

pub fn list_args(project: u64, filter: &ListFilter) -> Vec<String> {
    let mut argv = command("list", project);
    argv.extend(source(filter.source.as_deref().unwrap_or_default()));
    argv.extend(filter.closed.then(|| "--closed".to_owned()));
    if filter.mine {
        argv.extend(["--assignee", "me"].map(str::to_owned));
    }
    argv.extend(filter.cached.then(|| "--cached".to_owned()));
    argv.push("--json".to_owned());
    argv
}

pub fn read_args(project: u64, source_id: &str, key: &str) -> Vec<String> {
    let mut argv = command("read", project);
    argv.extend(source(source_id));
    argv.extend(["--json", "--", key.trim()].map(str::to_owned));
    argv
}

fn command(action: &str, project: u64) -> Vec<String> {
    ["issue", action, "--project", &project.to_string()].map(str::to_owned).into()
}

fn source(id: &str) -> Option<String> {
    let id = id.trim();
    (!id.is_empty()).then(|| format!("--source={id}"))
}

#[cfg(test)]
mod tests;
