use std::fmt;
use std::net::{IpAddr, Ipv6Addr};

use tokio_tungstenite::tungstenite::http::Uri;

use super::name::LONGEST_NAME;
use super::route::{Address, Route, RouteError, RouteUrl, Token, address, checked_token, token, values};

const UNRESERVED: &[u8] = b"-._~";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCode {
    first: String,
    also: Vec<String>,
    port: u16,
    token: Token,
    name: Option<String>,
}

impl MachineCode {
    pub fn new(first: IpAddr, port: u16, token: &str) -> Result<Self, RouteError> {
        if port == 0 {
            return Err(RouteError::Port);
        }
        let token = checked_token(token)?;
        Ok(Self { first: first.to_string(), also: Vec::new(), port, token, name: None })
    }

    #[must_use]
    pub fn with_also(mut self, ip: IpAddr) -> Self {
        self.add_also(ip.to_string());
        self
    }

    #[must_use]
    pub fn with_name(mut self, name: &str) -> Self {
        let name: String = name.trim().chars().take(LONGEST_NAME).collect();
        self.name = Some(name.trim_end().to_owned()).filter(|name| !name.is_empty());
        self
    }

    pub fn parse(text: &str) -> Result<Self, RouteError> {
        let uri: Uri = text.trim().parse().map_err(|_| RouteError::NotAUrl)?;
        let Address { host, port } = address(&uri)?;
        let query = uri.query();
        let mut code = Self { first: host, also: Vec::new(), port, token: token(query)?, name: name(query)? };
        for value in values(query, "also") {
            code.add_also(also(value)?);
        }
        Ok(code)
    }

    pub fn routes(&self) -> Vec<Route> {
        self.hosts().map(|host| self.route(host)).collect()
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn add_also(&mut self, host: String) {
        if self.hosts().all(|known| *known != host) {
            self.also.push(host);
        }
    }

    fn hosts(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.first).chain(&self.also)
    }

    fn route(&self, host: &str) -> Route {
        let address = Address { host: host.to_owned(), port: self.port };
        Route::Bridge { url: RouteUrl { address, token: Some(self.token.clone()) } }
    }
}

impl fmt::Display for MachineCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first = Address { host: self.first.clone(), port: self.port };
        write!(formatter, "{first}?token={}", self.token.as_str())?;
        for host in &self.also {
            write!(formatter, "&also={host}")?;
        }
        match &self.name {
            Some(name) => write!(formatter, "&name={}", encoded(name)),
            None => Ok(()),
        }
    }
}

fn also(value: &str) -> Result<String, RouteError> {
    let valid = if value.contains(':') {
        value.parse::<Ipv6Addr>().is_ok()
    } else {
        value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.')
    };
    match (value.is_empty(), valid) {
        (true, _) => Err(RouteError::AlsoEmpty),
        (false, true) => Ok(value.to_owned()),
        (false, false) => Err(RouteError::AlsoInvalid),
    }
}

fn name(query: Option<&str>) -> Result<Option<String>, RouteError> {
    match values(query, "name").as_slice() {
        [] => Ok(None),
        [value] => decoded(value).map(|name| Some(name.trim().to_owned()).filter(|name| !name.is_empty())),
        _ => Err(RouteError::NameTwice),
    }
}

fn encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || UNRESERVED.contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

fn decoded(text: &str) -> Result<String, RouteError> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        rest = tail;
        if byte == b'%' {
            let (hex, after) = tail.split_at_checked(2).ok_or(RouteError::NameEncoding)?;
            bytes.push(hex_byte(hex).ok_or(RouteError::NameEncoding)?);
            rest = after;
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).map_err(|_| RouteError::NameEncoding)
}

fn hex_byte(hex: &[u8]) -> Option<u8> {
    let digits = std::str::from_utf8(hex).ok().filter(|digits| digits.bytes().all(|byte| byte.is_ascii_hexdigit()))?;
    u8::from_str_radix(digits, 16).ok()
}

#[cfg(test)]
mod tests;
