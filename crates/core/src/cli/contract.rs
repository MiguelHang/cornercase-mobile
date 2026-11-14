use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Error;
use crate::transport::json_error;

pub mod boot;
#[cfg(test)]
pub(crate) mod fixtures;

pub const CONTRACT: u32 = 1;

const OLD_SERVER: &str = "has no `contract` command";
const OLD_CLI: &str = "unrecognized subcommand 'contract'";
const SINCE: &[(Kind, &str, &str)] =
    &[(Kind::Feature, "cli-endpoint", "0.13.78"), (Kind::Operation, "app-routes", "0.13.78")];

pub const REQUIRED: Required = Required {
    operations: &[
        "status",
        "watch",
        "app",
        "events",
        "last-message",
        "send",
        "keys",
        "mark",
        "paste-file",
        "view",
        "surface",
        "redraw",
        "input",
        "scroll",
        "view-set",
        "start",
        "account-list",
        "issue-sources",
        "issue-list",
        "issue-read",
        "changes",
        "changes-file",
        "changes-viewed",
        "files-list",
        "files-read",
        "files-search",
        "flow-list",
        "flow-run",
        "close-agent",
        "app-routes",
    ],
    features: &[
        "boot",
        "notices",
        "views",
        "surface-patches",
        "machine-start",
        "issues",
        "changes",
        "files",
        "accounts",
        "account-switch",
        "close-agent-tabs",
        "cli-endpoint",
    ],
    streams: &[("events", 1), ("watch", 1), ("app", 1), ("surface", 1), ("view", 1), ("changes", 1), ("files", 1)],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Required {
    pub operations: &'static [&'static str],
    pub features: &'static [&'static str],
    pub streams: &'static [(&'static str, u32)],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Operation,
    Feature,
    Stream,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Missing {
    pub name: String,
    pub kind: Kind,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Descriptor {
    pub version: String,
    pub contract: u32,
    pub boot: String,
    pub operations: Vec<Operation>,
    pub features: Vec<String>,
    pub streams: Vec<Stream>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Operation {
    pub name: String,
    pub available: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Stream {
    pub name: String,
    pub version: u32,
}

impl Descriptor {
    pub fn from_value(value: Value) -> Result<Self, Error> {
        Self::read(Self::deserialize(value))
    }

    fn read(result: Result<Self, serde_json::Error>) -> Result<Self, Error> {
        result.map_err(|error| json_error(&error))?.checked()
    }

    fn checked(self) -> Result<Self, Error> {
        match self.contract {
            0 => Err(Error::Outdated),
            server if server > CONTRACT => Err(Error::ContractTooNew { server, app: CONTRACT }),
            _ => Ok(self),
        }
    }
}

impl FromStr for Descriptor {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        Self::read(serde_json::from_str(text))
    }
}

impl Missing {
    fn new(name: &str, kind: Kind, reason: Option<String>) -> Self {
        Self { name: name.to_owned(), kind, reason }
    }

    pub fn since(&self) -> Option<&'static str> {
        SINCE.iter().find(|(kind, name, _)| *kind == self.kind && *name == self.name).map(|(_, _, version)| *version)
    }
}

pub fn missing(descriptor: &Descriptor) -> Vec<Missing> {
    REQUIRED.missing(descriptor)
}

impl Required {
    fn missing(&self, descriptor: &Descriptor) -> Vec<Missing> {
        let operations = self.operations.iter().filter_map(|name| missing_operation(descriptor, name));
        let features = self
            .features
            .iter()
            .filter(|name| !descriptor.features.iter().any(|feature| feature == *name))
            .map(|name| Missing::new(name, Kind::Feature, None));
        let streams = self
            .streams
            .iter()
            .filter(|(name, version)| {
                !descriptor.streams.iter().any(|stream| stream.name == *name && stream.version == *version)
            })
            .map(|(name, _)| Missing::new(name, Kind::Stream, None));
        operations.chain(features).chain(streams).collect()
    }
}

fn missing_operation(descriptor: &Descriptor, name: &str) -> Option<Missing> {
    match descriptor.operations.iter().find(|operation| operation.name == name) {
        Some(Operation { available: true, .. }) => None,
        Some(operation) => Some(Missing::new(name, Kind::Operation, operation.reason.clone())),
        None => Some(Missing::new(name, Kind::Operation, None)),
    }
}

pub fn outdated(error: Error) -> Error {
    match error {
        Error::Failed { stderr, .. } if stderr.contains(OLD_SERVER) || stderr.contains(OLD_CLI) => Error::Outdated,
        other => other,
    }
}

#[cfg(test)]
mod tests;
