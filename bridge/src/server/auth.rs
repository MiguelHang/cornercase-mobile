use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_tungstenite::tungstenite::handshake::server::{Callback, ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::{StatusCode, Uri};

use super::log;

const PATH: &str = "/";
const KEY: &str = "token";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    Path,
    NoToken,
    EmptyToken,
    TokenTwice,
    WrongToken,
    Busy,
}

impl Refusal {
    fn status(self) -> StatusCode {
        match self {
            Self::Path => StatusCode::NOT_FOUND,
            Self::NoToken | Self::EmptyToken | Self::TokenTwice | Self::WrongToken => StatusCode::UNAUTHORIZED,
            Self::Busy => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn reason(self) -> &'static str {
        match self {
            Self::Path => "not the path /",
            Self::NoToken => "no token",
            Self::EmptyToken => "an empty token",
            Self::TokenTwice => "more than one token",
            Self::WrongToken => "a wrong token",
            Self::Busy => "too many connections",
        }
    }
}

fn check(uri: &Uri, token: &str) -> Result<(), Refusal> {
    if uri.path() != PATH {
        return Err(Refusal::Path);
    }
    let mut given = uri.query().unwrap_or_default().split('&').filter_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        (key == KEY).then_some(value)
    });
    match (given.next(), given.next()) {
        (None, _) => Err(Refusal::NoToken),
        (Some(_), Some(_)) => Err(Refusal::TokenTwice),
        (Some(""), None) => Err(Refusal::EmptyToken),
        (Some(value), None) if same(value, token) => Ok(()),
        (Some(_), None) => Err(Refusal::WrongToken),
    }
}

fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub(super) struct Gate<'a> {
    pub(super) token: &'a str,
    pub(super) peer: SocketAddr,
    pub(super) permits: &'a Arc<Semaphore>,
    pub(super) permit: &'a mut Option<OwnedSemaphorePermit>,
}

impl Gate<'_> {
    fn admit(self, uri: &Uri) -> Result<(), Refusal> {
        check(uri, self.token)?;
        *self.permit = Some(Arc::clone(self.permits).try_acquire_owned().map_err(|_| Refusal::Busy)?);
        Ok(())
    }
}

impl Callback for Gate<'_> {
    fn on_request(self, request: &Request, response: Response) -> Result<Response, ErrorResponse> {
        let peer = self.peer;
        match self.admit(request.uri()) {
            Ok(()) => Ok(response),
            Err(refusal) => {
                log(format_args!("refused {peer}: {}", refusal.reason()));
                Err(answer(refusal.status()))
            }
        }
    }
}

fn answer(status: StatusCode) -> ErrorResponse {
    let mut response = ErrorResponse::new(None);
    *response.status_mut() = status;
    response
}

#[cfg(test)]
mod tests;
