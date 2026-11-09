use std::str::FromStr;

use serde::Deserialize;
use serde_json::Value;

use crate::error::Error;
use crate::transport::json_error;

pub const CONTRACT: u32 = 1;

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

impl FromStr for Descriptor {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        serde_json::from_str::<Self>(text).map_err(|error| json_error(&error))?.checked()
    }
}

#[cfg(test)]
mod tests;
