use std::fmt::Display;

use cornercase_core::error::Error;
use cornercase_core::machines::route::RouteError;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub(crate) struct CommandError {
    pub(crate) code: ErrorCode,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ErrorCode {
    Unauthorized,
    Dropped,
    OutputClosed,
    Rejected,
    Timeout,
    Connect,
    Protocol,
    Failed,
    StaleBoot,
    StreamEnded,
    Outdated,
    ContractTooNew,
    Unanswered,
    SettleTimeout,
    InputRejected,
    UnknownKey,
    NoKeys,
    ImageEmpty,
    ImageTooLarge,
    Picker,
    Json,
    Store,
    Secrets,
    NoToken,
    UnknownMachine,
    UnknownRoute,
    LastRoute,
    NameEmpty,
    NameTooLong,
    NameTaken,
    Identity,
    Random,
    RouteNotAUrl,
    RouteScheme,
    RouteUserInfo,
    RouteNoHost,
    RoutePort,
    RoutePath,
    RouteNoToken,
    RouteEmptyToken,
    RouteTokenTwice,
    RouteTokenCharacters,
    RouteAlsoEmpty,
    RouteAlsoInvalid,
    RouteNameEncoding,
    RouteNameTwice,
    NoDataDir,
    MachinesNotLoaded,
    NoIdentity,
    NotConnected,
    NotSent,
    NoDigest,
    TooLargeHere,
    AttachedFull,
    AttachedGone,
}

impl CommandError {
    fn new(code: ErrorCode, message: impl Display) -> Self {
        Self { code, message: message.to_string() }
    }

    pub(crate) fn no_data_dir(reason: impl Display) -> Self {
        Self::new(ErrorCode::NoDataDir, format!("the app has no folder for its data: {reason}"))
    }

    pub(crate) fn machines_not_loaded(error: &Error) -> Self {
        Self::new(ErrorCode::MachinesNotLoaded, format!("the saved machines are not loaded: {error}"))
    }

    pub(crate) fn no_identity(error: &Error) -> Self {
        Self::new(ErrorCode::NoIdentity, format!("the app has no identity: {error}"))
    }

    pub(crate) fn not_connected() -> Self {
        Self::new(ErrorCode::NotConnected, "no machine is connected")
    }

    pub(crate) fn not_sent() -> Self {
        Self::new(ErrorCode::NotSent, "nothing was sent")
    }

    pub(crate) fn no_digest() -> Self {
        Self::new(ErrorCode::NoDigest, "marking a file viewed needs the digest of the version shown")
    }

    pub(crate) fn too_large_here() -> Self {
        Self::new(ErrorCode::TooLargeHere, "Too large to show on the phone")
    }

    pub(crate) fn too_many_attached(most: usize) -> Self {
        Self::new(ErrorCode::AttachedFull, format!("at most {most} images can wait to be sent"))
    }

    pub(crate) fn attached_too_large() -> Self {
        Self::new(ErrorCode::AttachedFull, "the images waiting to be sent take too much memory")
    }

    pub(crate) fn attached_gone() -> Self {
        Self::new(ErrorCode::AttachedGone, "the image is no longer waiting to be sent")
    }

    pub(crate) fn maybe_sent() -> Self {
        Self::new(ErrorCode::Unanswered, "it may have been sent, but no answer came back")
    }

    pub(crate) fn reason(error: &Error) -> Self {
        match error {
            Error::Failed { stderr, .. } if !said(stderr).is_empty() => Self::new(ErrorCode::Failed, said(stderr)),
            _ => Self::from(error),
        }
    }
}

fn said(stderr: &str) -> &str {
    let stderr = stderr.trim();
    stderr.strip_prefix("Error:").map_or(stderr, str::trim_start)
}

impl From<&Error> for CommandError {
    fn from(error: &Error) -> Self {
        Self::new(core_code(error), error)
    }
}

impl From<Error> for CommandError {
    fn from(error: Error) -> Self {
        Self::from(&error)
    }
}

impl From<RouteError> for CommandError {
    fn from(error: RouteError) -> Self {
        Self::new(route_code(error), error)
    }
}

fn core_code(error: &Error) -> ErrorCode {
    match error {
        Error::Unauthorized => ErrorCode::Unauthorized,
        Error::Dropped => ErrorCode::Dropped,
        Error::OutputClosed => ErrorCode::OutputClosed,
        Error::Rejected { .. } => ErrorCode::Rejected,
        Error::Timeout => ErrorCode::Timeout,
        Error::Connect(_) => ErrorCode::Connect,
        Error::Protocol(_) => ErrorCode::Protocol,
        Error::Failed { .. } => ErrorCode::Failed,
        Error::StaleBoot(_) => ErrorCode::StaleBoot,
        Error::StreamEnded => ErrorCode::StreamEnded,
        Error::Outdated => ErrorCode::Outdated,
        Error::ContractTooNew { .. } => ErrorCode::ContractTooNew,
        Error::Unanswered => ErrorCode::Unanswered,
        Error::SettleTimeout => ErrorCode::SettleTimeout,
        Error::InputRejected(_) => ErrorCode::InputRejected,
        Error::UnknownKey(_) => ErrorCode::UnknownKey,
        Error::NoKeys => ErrorCode::NoKeys,
        Error::ImageEmpty => ErrorCode::ImageEmpty,
        Error::ImageTooLarge => ErrorCode::ImageTooLarge,
        Error::Picker(_) => ErrorCode::Picker,
        Error::Json(_) => ErrorCode::Json,
        Error::Store(_) => ErrorCode::Store,
        Error::Secrets(_) => ErrorCode::Secrets,
        Error::NoToken => ErrorCode::NoToken,
        Error::UnknownMachine => ErrorCode::UnknownMachine,
        Error::UnknownRoute => ErrorCode::UnknownRoute,
        Error::LastRoute => ErrorCode::LastRoute,
        Error::NameEmpty => ErrorCode::NameEmpty,
        Error::NameTooLong => ErrorCode::NameTooLong,
        Error::NameTaken => ErrorCode::NameTaken,
        Error::Identity => ErrorCode::Identity,
        Error::Random => ErrorCode::Random,
    }
}

fn route_code(error: RouteError) -> ErrorCode {
    match error {
        RouteError::NotAUrl => ErrorCode::RouteNotAUrl,
        RouteError::Scheme => ErrorCode::RouteScheme,
        RouteError::UserInfo => ErrorCode::RouteUserInfo,
        RouteError::NoHost => ErrorCode::RouteNoHost,
        RouteError::Port => ErrorCode::RoutePort,
        RouteError::Path => ErrorCode::RoutePath,
        RouteError::NoToken => ErrorCode::RouteNoToken,
        RouteError::EmptyToken => ErrorCode::RouteEmptyToken,
        RouteError::TokenTwice => ErrorCode::RouteTokenTwice,
        RouteError::TokenCharacters => ErrorCode::RouteTokenCharacters,
        RouteError::AlsoEmpty => ErrorCode::RouteAlsoEmpty,
        RouteError::AlsoInvalid => ErrorCode::RouteAlsoInvalid,
        RouteError::NameEncoding => ErrorCode::RouteNameEncoding,
        RouteError::NameTwice => ErrorCode::RouteNameTwice,
    }
}

#[cfg(test)]
mod tests;
