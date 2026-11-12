use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::transport::json_error;

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Place {
    Workspace {
        #[cfg_attr(feature = "specta", specta(type = u32))]
        workspace: u64,
    },
    Worktree {
        #[cfg_attr(feature = "specta", specta(type = u32))]
        project: u64,
        branch: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AccountPick {
    pub agent: String,
    pub account: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StartAgent {
    pub place: Place,
    pub agent: String,
    pub model: String,
    pub prompt: String,
    pub accounts: Vec<AccountPick>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StartIssue {
    pub place: Place,
    pub agent: String,
    pub model: String,
    pub key: String,
    pub source: String,
    pub issue_branch: String,
    pub accounts: Vec<AccountPick>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Started {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub workspace: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub tab: u64,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub pane: u64,
    pub warning: Option<String>,
}

impl Place {
    fn target(&self) -> [String; 2] {
        match self {
            Self::Workspace { workspace } => ["--workspace".to_owned(), workspace.to_string()],
            Self::Worktree { project, .. } => ["--project".to_owned(), project.to_string()],
        }
    }

    fn branch(&self) -> &str {
        match self {
            Self::Workspace { .. } => "",
            Self::Worktree { branch, .. } => branch,
        }
    }
}

impl StartAgent {
    pub fn argv(&self) -> Vec<String> {
        let mut argv = words(&["start", self.agent.trim()]);
        argv.extend(self.place.target());
        argv.extend(optional("--worktree", self.place.branch()));
        argv.extend(optional("--model", &self.model));
        argv.push(format!("--prompt={}", self.prompt.trim()));
        argv.extend(account_args(&self.accounts));
        argv.extend(words(&["--placed", "--json"]));
        argv
    }
}

impl StartIssue {
    pub fn argv(&self) -> Vec<String> {
        let mut argv = words(&["start", self.agent.trim()]);
        argv.extend(self.place.target());
        argv.push(format!("--issue={}", self.key.trim()));
        argv.extend(optional("--source", &self.source));
        argv.extend(optional("--worktree", other_branch(self.place.branch(), &self.issue_branch)));
        argv.extend(optional("--model", &self.model));
        argv.extend(account_args(&self.accounts));
        argv.extend(words(&["--placed", "--json"]));
        argv
    }
}

pub fn started(text: &str) -> Result<Started, Error> {
    serde_json::from_str(text).map_err(|error| json_error(&error))
}

pub(crate) fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|&word| word.to_owned()).collect()
}

pub(crate) fn other_branch<'a>(branch: &'a str, issue_branch: &str) -> &'a str {
    let branch = branch.trim();
    if branch == issue_branch.trim() { "" } else { branch }
}

pub(crate) fn optional(name: &str, value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| format!("{name}={value}"))
}

pub(crate) fn account_args(accounts: &[AccountPick]) -> impl Iterator<Item = String> {
    accounts.iter().map(|pick| format!("--account={}={}", pick.agent.trim(), pick.account.trim()))
}

#[cfg(test)]
mod tests;
