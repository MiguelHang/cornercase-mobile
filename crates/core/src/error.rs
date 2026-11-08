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
    #[error("no answer is scripted for {0:?}")]
    Unscripted(Vec<String>),
}

#[cfg(test)]
mod tests;
