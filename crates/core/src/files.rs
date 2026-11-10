use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(|error| failed("cannot create the folder", &error))?;
    }
    let temporary = temporary(path);
    or_remove(write(&temporary, bytes), &temporary)
        .map_err(|error| failed("cannot write the temporary file", &error))?;
    or_remove(fs::rename(&temporary, path), &temporary).map_err(|error| failed("cannot replace the file", &error))
}

pub(crate) fn temporary(path: &Path) -> PathBuf {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!(".{name}.tmp"))
}

fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn or_remove<T>(result: io::Result<T>, temporary: &Path) -> io::Result<T> {
    if result.is_err() {
        fs::remove_file(temporary).ok();
    }
    result
}

fn failed(what: &str, error: &io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{what}: {error}"))
}

#[cfg(test)]
mod tests;
