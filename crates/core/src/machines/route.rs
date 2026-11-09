use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio_tungstenite::tungstenite::http::Uri;

use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Route {
    Bridge { url: BridgeUrl },
}

impl Route {
    pub fn address(&self) -> &Address {
        match self {
            Self::Bridge { url } => &url.address,
        }
    }

    pub fn connection_url(&self) -> Result<String, Error> {
        match self {
            Self::Bridge { url } => url.connection_url(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeUrl {
    pub address: Address,
    pub token: Option<Token>,
}

impl BridgeUrl {
    fn connection_url(&self) -> Result<String, Error> {
        let token = self.token.as_ref().ok_or(Error::NoToken)?;
        Ok(format!("{}?token={}", self.address, token.0))
    }
}

impl Serialize for BridgeUrl {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.address.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BridgeUrl {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self { address: Address::deserialize(deserializer)?, token: None })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address {
    pub host: String,
    pub port: u16,
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.host.contains(':') {
            write!(formatter, "ws://[{}]:{}/", self.host, self.port)
        } else {
            write!(formatter, "ws://{}:{}/", self.host, self.port)
        }
    }
}

impl Serialize for Address {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        saved_address(&text).ok_or_else(|| D::Error::custom("a saved route is not a ws://host:port/ address"))
    }
}

fn saved_address(text: &str) -> Option<Address> {
    let uri: Uri = text.parse().ok()?;
    let host = uri.host()?.trim_start_matches('[').trim_end_matches(']');
    let port = uri.port_u16().filter(|port| *port != 0)?;
    let bare = uri.scheme_str() == Some("ws") && uri.path() == "/" && uri.query().is_none() && !host.is_empty();
    bare.then(|| Address { host: host.to_owned(), port })
}

#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Token").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
