use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

pub const CONFIG_HOME: &str = "XDG_CONFIG_HOME";
pub const HOME: &str = "HOME";
const FOLDER: &str = "cornercase-bridge";
const FILE: &str = "token";
const NEW_TOKEN: &str = "--new-token";
const BYTES: usize = 32;
const TEMP_BYTES: usize = 9;
const FOLDER_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;
const SHARED_MODE: u32 = 0o077;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoHome;

impl fmt::Display for NoHome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{HOME} is not set to an absolute path, so there is no place for the token")
    }
}

impl std::error::Error for NoHome {}

#[derive(Debug)]
pub enum Error {
    Io { path: PathBuf, error: io::Error },
    Random(getrandom::Error),
    Invalid { path: PathBuf },
    Shared { path: PathBuf, mode: u32 },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, error } => write!(f, "{}: {error}", path.display()),
            Self::Random(error) => write!(f, "no random bytes for the token: {error}"),
            Self::Invalid { path } => {
                write!(f, "{} does not hold a token; run with {NEW_TOKEN} to make a new one", path.display())
            }
            Self::Shared { path, mode } => write!(
                f,
                "{} can be read by other users (mode {mode:03o}); run with {NEW_TOKEN} to make a new one",
                path.display()
            ),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    fn io(path: &Path) -> impl FnOnce(io::Error) -> Self {
        let path = path.to_path_buf();
        move |error| Self::Io { path, error }
    }
}

pub fn path(config_home: Option<&OsStr>, home: Option<&OsStr>) -> Result<PathBuf, NoHome> {
    let base = match absolute(config_home) {
        Some(config_home) => config_home.to_path_buf(),
        None => absolute(home).ok_or(NoHome)?.join(".config"),
    };
    Ok(base.join(FOLDER).join(FILE))
}

pub fn load_or_create(path: &Path) -> Result<String, Error> {
    match File::open(path) {
        Ok(file) => read(path, file),
        Err(error) if error.kind() == ErrorKind::NotFound => create(path),
        Err(error) => Err(Error::io(path)(error)),
    }
}

pub fn rotate(path: &Path) -> Result<String, Error> {
    let token = fresh()?;
    let temp = written(path, &token)?;
    or_remove(fs::rename(&temp, path), &temp).map_err(Error::io(path))?;
    Ok(token)
}

fn absolute(variable: Option<&OsStr>) -> Option<&Path> {
    variable.map(Path::new).filter(|path| path.is_absolute())
}

fn create(path: &Path) -> Result<String, Error> {
    let token = fresh()?;
    let temp = written(path, &token)?;
    let linked = fs::hard_link(&temp, path);
    fs::remove_file(&temp).map_err(Error::io(&temp))?;
    match linked {
        Ok(()) => Ok(token),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            read(path, File::open(path).map_err(Error::io(path))?)
        }
        Err(error) => Err(Error::io(path)(error)),
    }
}

fn read(path: &Path, mut file: File) -> Result<String, Error> {
    let mode = file.metadata().map_err(Error::io(path))?.permissions().mode() & 0o777;
    if mode & SHARED_MODE != 0 {
        return Err(Error::Shared { path: path.to_path_buf(), mode });
    }
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).map_err(Error::io(path))?;
    let token = contents.strip_suffix(b"\n").unwrap_or(&contents);
    str::from_utf8(token)
        .ok()
        .filter(|token| valid(token))
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid { path: path.to_path_buf() })
}

fn valid(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn fresh() -> Result<String, Error> {
    random::<BYTES>()
}

fn random<const N: usize>() -> Result<String, Error> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(Error::Random)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn written(path: &Path, token: &str) -> Result<PathBuf, Error> {
    if let Some(folder) = path.parent() {
        DirBuilder::new().recursive(true).mode(FOLDER_MODE).create(folder).map_err(Error::io(folder))?;
    }
    let temp = path.with_file_name(format!(".{FILE}-{}", random::<TEMP_BYTES>()?));
    or_remove(write(&temp, token), &temp).map_err(Error::io(&temp))?;
    Ok(temp)
}

fn write(temp: &Path, token: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).mode(FILE_MODE).open(temp)?;
    file.write_all(format!("{token}\n").as_bytes())?;
    file.sync_all()
}

fn or_remove<T>(result: io::Result<T>, temp: &Path) -> io::Result<T> {
    if result.is_err() {
        fs::remove_file(temp).ok();
    }
    result
}

#[cfg(test)]
mod tests;
