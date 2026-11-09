use std::str::FromStr;

use serde::Deserialize;
use serde_json::Value;

use crate::error::Error;
use crate::transport::json_error;

pub mod boot;

pub const CONTRACT: u32 = 1;

const OLD_SERVER: &str = "has no `contract` command";
const OLD_CLI: &str = "unrecognized subcommand 'contract'";

pub const REQUIRED: Required = Required {
    operations: &[
        "status",
        "watch",
        "app",
        "events",
        "last-message",
        "send",
        "keys",
        "paste-file",
        "view",
        "surface",
        "redraw",
        "input",
        "scroll",
        "view-set",
        "start",
        "new-workspace",
        "issue-sources",
        "issue-list",
        "issue-read",
        "changes",
        "changes-file",
        "changes-viewed",
        "files-list",
        "files-read",
        "files-search",
    ],
    features: &["boot", "views", "surface-patches", "machine-start", "issues", "changes", "files"],
    streams: &[("events", 1), ("watch", 1), ("app", 1), ("surface", 1), ("view", 1), ("changes", 1), ("files", 1)],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Required {
    pub operations: &'static [&'static str],
    pub features: &'static [&'static str],
    pub streams: &'static [(&'static str, u32)],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Operation,
    Feature,
    Stream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
        Self::deserialize(value).map_err(|error| json_error(&error))?.checked()
    }

    fn checked(self) -> Result<Self, Error> {
        match self.contract {
            0 => Err(Error::Outdated),
            server if server > CONTRACT => Err(Error::ContractTooNew { server, app: CONTRACT }),
            _ => Ok(self),
        }
    }
}

impl Missing {
    fn new(name: &str, kind: Kind, reason: Option<String>) -> Self {
        Self { name: name.to_owned(), kind, reason }
    }
}

pub fn missing(descriptor: &Descriptor) -> Vec<Missing> {
    let operations = REQUIRED.operations.iter().filter_map(|name| missing_operation(descriptor, name));
    let features = REQUIRED
        .features
        .iter()
        .filter(|name| !descriptor.features.iter().any(|feature| feature == *name))
        .map(|name| Missing::new(name, Kind::Feature, None));
    let streams = REQUIRED
        .streams
        .iter()
        .filter(|(name, version)| {
            !descriptor.streams.iter().any(|stream| stream.name == *name && stream.version == *version)
        })
        .map(|(name, _)| Missing::new(name, Kind::Stream, None));
    operations.chain(features).chain(streams).collect()
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

impl FromStr for Descriptor {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        serde_json::from_str::<Self>(text).map_err(|error| json_error(&error))?.checked()
    }
}

#[cfg(test)]
mod tests;
