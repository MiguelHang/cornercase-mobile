use std::net::Ipv6Addr;

use tokio_tungstenite::tungstenite::http::Uri;

use super::route::{Address, Route, RouteError, Token, address, values};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCode {
    first: String,
    also: Vec<String>,
    port: u16,
    code: Token,
    name: Option<String>,
}

impl MachineCode {
    pub fn parse(text: &str) -> Result<Self, RouteError> {
        let uri: Uri = text.trim().parse().map_err(|_| RouteError::NotAUrl)?;
        let Address { host, port } = address(&uri)?;
        let query = uri.query();
        let mut code = Self { first: host, also: Vec::new(), port, code: pairing_code(query)?, name: name(query)? };
        for value in values(query, "also") {
            code.add_also(also(value)?);
        }
        Ok(code)
    }

    pub fn cli_routes(&self, secret: &Token) -> Vec<Route> {
        self.addresses().into_iter().map(|address| Route::cli(address, Some(secret.clone()))).collect()
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub(crate) fn pairing_code(&self) -> &Token {
        &self.code
    }

    pub(crate) fn addresses(&self) -> Vec<Address> {
        self.hosts().map(|host| self.address(host)).collect()
    }

    fn add_also(&mut self, host: String) {
        if self.hosts().all(|known| *known != host) {
            self.also.push(host);
        }
    }

    fn hosts(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.first).chain(&self.also)
    }

    fn address(&self, host: &str) -> Address {
        Address { host: host.to_owned(), port: self.port }
    }
}

fn pairing_code(query: Option<&str>) -> Result<Token, RouteError> {
    match values(query, "code").as_slice() {
        [] => Err(RouteError::NoCode),
        [value] => checked_code(value),
        _ => Err(RouteError::CodeTwice),
    }
}

fn checked_code(value: &str) -> Result<Token, RouteError> {
    if value.is_empty() {
        Err(RouteError::EmptyCode)
    } else if value.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')) {
        Ok(Token::new(value.to_owned()))
    } else {
        Err(RouteError::CodeCharacters)
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
