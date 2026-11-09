#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("the bridge refused the token")]
    Unauthorized,
    #[error("the connection to the bridge dropped before the command finished")]
    Dropped,
    #[error("nothing listens to the command's output any more")]
    OutputClosed,
    #[error("the bridge answered with HTTP {status}")]
    Rejected { status: u16 },
    #[error("the bridge did not answer in time")]
    Timeout,
    #[error("cannot reach the bridge: {0}")]
    Connect(String),
    #[error("the bridge sent an unexpected message: {0}")]
    Protocol(String),
    #[error("cornercase exited with code {code}: {stderr}")]
    Failed { code: i32, stderr: String },
    #[error("cornercase's output is not the expected JSON: {0}")]
    Json(String),
    #[error("cannot read or write the saved machines: {0}")]
    Store(String),
    #[error("the secret store failed: {0}")]
    Secrets(String),
    #[error("the route has no saved token; scan the bridge's code again")]
    NoToken,
    #[error("no saved machine has that id")]
    UnknownMachine,
    #[error("the machine has no route at that address")]
    UnknownRoute,
    #[error("the saved identity is not a 32-byte Ed25519 seed")]
    Identity,
}

#[cfg(test)]
mod tests;
