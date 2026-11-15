use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio_tungstenite::tungstenite::http::Uri;
use tokio_tungstenite::tungstenite::http::uri::Authority;

use crate::error::Error;

const DEFAULT_PORT: u16 = 80;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Route {
    Bridge { url: RouteUrl },
    Cli { url: RouteUrl },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RouteError {
    #[error("the text is not a machine code")]
    NotAUrl,
    #[error("the code does not start with ws://")]
    Scheme,
    #[error("the code has a user name or a password")]
    UserInfo,
    #[error("the code has no host")]
    NoHost,
    #[error("the code's port is not a number from 1 to 65535")]
    Port,
    #[error("the code's path is not /")]
    Path,
    #[error("the code has no token")]
    NoToken,
    #[error("the code's token is empty")]
    EmptyToken,
    #[error("the code has more than one token")]
    TokenTwice,
    #[error("the code's token has characters other than letters, digits, - and _")]
    TokenCharacters,
    #[error("the code has no pairing code")]
    NoCode,
    #[error("this is a code of the old bridge: scan the one in cornercase's Settings › Remote › Phone app")]
    BridgeCode,
    #[error("the code's pairing code is empty")]
    EmptyCode,
    #[error("the code has more than one pairing code")]
    CodeTwice,
    #[error("the code's pairing code has characters other than 0-9 and a-f")]
    CodeCharacters,
    #[error("the code has an empty also= address")]
    AlsoEmpty,
    #[error("an also= address of the code is not a host name or an IP address without brackets")]
    AlsoInvalid,
    #[error("the code's name is not percent-encoded UTF-8")]
    NameEncoding,
    #[error("the code has more than one name")]
    NameTwice,
}

impl Route {
    pub fn parse(text: &str) -> Result<Self, RouteError> {
        let uri: Uri = text.trim().parse().map_err(|_| RouteError::NotAUrl)?;
        let address = address(&uri)?;
        let token = token(uri.query())?;
        Ok(Self::Bridge { url: RouteUrl { address, token: Some(token) } })
    }

    pub fn address(&self) -> &Address {
        &self.url().address
    }

    pub fn connection_url(&self) -> Result<String, Error> {
        match self {
            Self::Bridge { url } => url.bridge_url(),
            Self::Cli { url } => Ok(format!("{}cli", url.address)),
        }
    }

    pub(crate) fn bearer(&self) -> Result<Option<&str>, Error> {
        match self {
            Self::Bridge { .. } => Ok(None),
            Self::Cli { url } => url.token.as_ref().map(|secret| Some(secret.as_str())).ok_or(Error::NoToken),
        }
    }

    pub fn has_token(&self) -> bool {
        self.token().is_some()
    }

    pub(crate) fn token(&self) -> Option<&Token> {
        self.url().token.as_ref()
    }

    pub(crate) fn set_token(&mut self, token: Option<Token>) {
        match self {
            Self::Bridge { url } | Self::Cli { url } => url.token = token,
        }
    }

    pub(crate) fn at(&self, address: Address) -> Self {
        let url = RouteUrl { address, token: self.token().cloned() };
        match self {
            Self::Bridge { .. } => Self::Bridge { url },
            Self::Cli { .. } => Self::Cli { url },
        }
    }

    fn url(&self) -> &RouteUrl {
        match self {
            Self::Bridge { url } | Self::Cli { url } => url,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteUrl {
    pub(crate) address: Address,
    pub(crate) token: Option<Token>,
}

impl RouteUrl {
    fn bridge_url(&self) -> Result<String, Error> {
        let token = self.token.as_ref().ok_or(Error::NoToken)?;
        Ok(format!("{}?token={}", self.address, token.0))
    }
}

impl Serialize for RouteUrl {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.address.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RouteUrl {
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

pub(super) fn saved_address(text: &str) -> Option<Address> {
    let uri: Uri = text.parse().ok()?;
    address(&uri).ok().filter(|address| address.to_string() == text)
}

pub(super) fn address(uri: &Uri) -> Result<Address, RouteError> {
    if !uri.scheme_str().is_some_and(|scheme| scheme.eq_ignore_ascii_case("ws")) {
        return Err(RouteError::Scheme);
    }
    let authority = uri.authority().ok_or(RouteError::NotAUrl)?;
    if authority.as_str().contains('@') {
        return Err(RouteError::UserInfo);
    }
    let address = Address { host: host(authority)?, port: port(authority)? };
    path(uri)?;
    Ok(address)
}

fn host(authority: &Authority) -> Result<String, RouteError> {
    let host = authority.host().trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() { Err(RouteError::NoHost) } else { Ok(host.to_owned()) }
}

fn port(authority: &Authority) -> Result<u16, RouteError> {
    let text = authority.as_str();
    let after_host = text.rsplit_once(']').map_or(text, |(_, rest)| rest);
    let Some((_, digits)) = after_host.rsplit_once(':') else {
        return Ok(DEFAULT_PORT);
    };
    let number = digits.bytes().all(|byte| byte.is_ascii_digit()).then(|| digits.parse::<u16>().ok()).flatten();
    number.filter(|port| *port != 0).ok_or(RouteError::Port)
}

fn path(uri: &Uri) -> Result<(), RouteError> {
    let full = uri.path_and_query().map_or("", |path_and_query| path_and_query.as_str());
    let path = full.split_once('?').map_or(full, |(path, _)| path);
    if path == "/" { Ok(()) } else { Err(RouteError::Path) }
}

fn token(query: Option<&str>) -> Result<Token, RouteError> {
    match values(query, "token").as_slice() {
        [] => Err(RouteError::NoToken),
        [value] => checked_token(value),
        _ => Err(RouteError::TokenTwice),
    }
}

fn checked_token(value: &str) -> Result<Token, RouteError> {
    if value.is_empty() {
        Err(RouteError::EmptyToken)
    } else if base64url(value) {
        Ok(Token(value.to_owned()))
    } else {
        Err(RouteError::TokenCharacters)
    }
}

pub(super) fn values<'a>(query: Option<&'a str>, key: &str) -> Vec<&'a str> {
    let pairs = query.unwrap_or_default().split('&');
    pairs.filter_map(|pair| value(pair, key)).collect()
}

fn value<'a>(pair: &'a str, key: &str) -> Option<&'a str> {
    match pair.split_once('=') {
        Some((name, value)) => (name == key).then_some(value),
        None => (pair == key).then_some(""),
    }
}

fn base64url(text: &str) -> bool {
    text.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn fake(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Token").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
