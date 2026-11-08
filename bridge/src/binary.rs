use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rustix::fs::{Access, access};

pub const VARIABLE: &str = "CORNERCASE_BIN";
const OPTION: &str = "--cornercase";
const DEFAULT_NAME: &str = "cornercase";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Option,
    Variable,
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotFound {
    NotExecutable { origin: Origin, path: PathBuf },
    NotInPath { origin: Origin, name: OsString, path: Option<OsString> },
}

impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotExecutable { origin, path } => {
                write!(f, "{}{} does not exist or is not executable", source(*origin), path.display())
            }
            Self::NotInPath { origin, name, path: Some(path) } => {
                write!(f, "{}no executable `{}` in the PATH ({})", source(*origin), name.display(), path.display())
            }
            Self::NotInPath { origin, name, path: None } => {
                write!(f, "{}no executable `{}`: PATH is not set", source(*origin), name.display())
            }
        }
    }
}

impl std::error::Error for NotFound {}

fn source(origin: Origin) -> String {
    match origin {
        Origin::Option => format!("{OPTION}: "),
        Origin::Variable => format!("{VARIABLE}: "),
        Origin::Default => String::new(),
    }
}

pub fn find(
    option: Option<&Path>,
    variable: Option<&OsStr>,
    path: Option<&OsStr>,
    is_executable: impl Fn(&Path) -> bool,
) -> Result<PathBuf, NotFound> {
    let (origin, wanted) = match (option, variable.filter(|value| !value.is_empty())) {
        (Some(option), _) => (Origin::Option, option.as_os_str()),
        (None, Some(variable)) => (Origin::Variable, variable),
        (None, None) => (Origin::Default, OsStr::new(DEFAULT_NAME)),
    };
    if origin == Origin::Option || has_slash(wanted) {
        return at_path(origin, Path::new(wanted), is_executable);
    }
    in_path(origin, wanted, path, is_executable)
}

pub fn is_executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) && access(path, Access::EXEC_OK).is_ok()
}

fn has_slash(name: &OsStr) -> bool {
    name.as_encoded_bytes().contains(&b'/')
}

fn at_path(origin: Origin, path: &Path, is_executable: impl Fn(&Path) -> bool) -> Result<PathBuf, NotFound> {
    if is_executable(path) {
        Ok(path.to_path_buf())
    } else {
        Err(NotFound::NotExecutable { origin, path: path.to_path_buf() })
    }
}

fn in_path(
    origin: Origin,
    name: &OsStr,
    path: Option<&OsStr>,
    is_executable: impl Fn(&Path) -> bool,
) -> Result<PathBuf, NotFound> {
    path.and_then(|path| {
        std::env::split_paths(path)
            .filter(|directory| !directory.as_os_str().is_empty())
            .map(|directory| directory.join(name))
            .find(|candidate| is_executable(candidate))
    })
    .ok_or_else(|| NotFound::NotInPath { origin, name: name.to_owned(), path: path.map(OsStr::to_owned) })
}

#[cfg(test)]
mod tests;
