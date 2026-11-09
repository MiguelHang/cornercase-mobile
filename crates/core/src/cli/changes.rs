use serde::{Deserialize, Serialize};

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Uncommitted,
    Commits,
    All,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum State {
    #[default]
    Loading,
    Ready,
    Failed,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Fold {
    #[default]
    Open,
    Lockfile,
    Binary,
    Large,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Staged {
    No,
    Partly,
    Yes,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum LineKind {
    #[default]
    Context,
    Removed,
    Added,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct ChangedFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: Status,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub added: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub removed: u64,
    pub fold: Fold,
    pub viewed: bool,
    pub staged: Option<Staged>,
    pub digest: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct ChangedWorkspace {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub workspace: u64,
    pub base: Option<String>,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub added: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub removed: u64,
    pub files: Vec<ChangedFile>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Summary {
    pub boot: String,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub stream: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub revision: u64,
    pub scope: String,
    pub mode: Mode,
    pub state: State,
    pub error: Option<String>,
    pub workspaces: Vec<ChangedWorkspace>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Line {
    pub kind: LineKind,
    #[cfg_attr(feature = "specta", specta(type = Option<u32>))]
    pub old: Option<u64>,
    #[cfg_attr(feature = "specta", specta(type = Option<u32>))]
    pub new: Option<u64>,
    pub text: String,
    pub raw: Option<String>,
    #[cfg_attr(feature = "specta", specta(type = Vec<[u32; 2]>))]
    pub emphasis: Vec<[u64; 2]>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Hunk {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub old_start: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub new_start: u64,
    pub context: String,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Patch {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub workspace: u64,
    pub mode: Mode,
    #[serde(flatten)]
    pub file: ChangedFile,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mark {
    Seen { digest: String },
    Clear { mode: Mode },
}

impl Mode {
    pub fn arg(self) -> &'static str {
        match self {
            Self::Uncommitted => "uncommitted",
            Self::Commits => "commits",
            Self::All => "all",
        }
    }
}

pub fn follow_args(workspace: u64, mode: Mode) -> Vec<String> {
    words(&["changes", "list", "--follow", "--workspace", &workspace.to_string(), "--mode", mode.arg()])
}

pub fn file_args(workspace: u64, path: &str, mode: Mode) -> Vec<String> {
    let mut argv = words(&["changes", "file", "--workspace", &workspace.to_string(), "--mode", mode.arg()]);
    argv.extend(words(&["--json", "--", path]));
    argv
}

pub fn viewed_args(workspace: u64, path: &str, mark: &Mark) -> Vec<String> {
    let mut argv = words(&["changes", "viewed", "--workspace", &workspace.to_string()]);
    match mark {
        Mark::Seen { digest } => argv.push(format!("--digest={digest}")),
        Mark::Clear { mode } => argv.extend(words(&["--clear", "--mode", mode.arg()])),
    }
    argv.extend(words(&["--", path]));
    argv
}

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|&word| word.to_owned()).collect()
}

#[cfg(test)]
mod tests;
