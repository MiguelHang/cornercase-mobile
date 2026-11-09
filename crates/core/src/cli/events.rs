use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "Value")]
pub struct Event {
    pub time: String,
    pub project: Option<u64>,
    pub workspace: Option<u64>,
    pub tab: Option<u64>,
    pub pane: Option<u64>,
    pub what: What,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum What {
    Status {
        agent: String,
        from: Option<String>,
        to: Option<String>,
    },
    Opened {
        kind: Kind,
        parent: Option<u64>,
    },
    Closed {
        kind: Kind,
    },
    Exited {
        program: String,
    },
    Notice {
        token: String,
        agent: String,
        status: String,
        place: String,
        message: String,
        panes: Vec<u64>,
        machine: Option<String>,
    },
    Dropped {
        count: u64,
    },
    #[serde(skip)]
    Other(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Project,
    Workspace,
    Tab,
    Pane,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Head {
    time: String,
    project: Option<u64>,
    workspace: Option<u64>,
    tab: Option<u64>,
    pane: Option<u64>,
}

impl From<Value> for Event {
    fn from(value: Value) -> Self {
        let Head { time, project, workspace, tab, pane } = Head::deserialize(&value).unwrap_or_default();
        let what = What::deserialize(&value).unwrap_or(What::Other(value));
        Self { time, project, workspace, tab, pane, what }
    }
}

#[cfg(test)]
mod tests;
