use crate::error::Error;

const NAMES: [&str; 18] =
    ["esc", "tab", "up", "down", "left", "right", "enter", "ctrl+c", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

pub fn args(pane: u64, key: &str) -> Result<Vec<String>, Error> {
    if !NAMES.contains(&key) {
        return Err(Error::UnknownKey(key.to_owned()));
    }
    Ok(["keys", "--pane", &pane.to_string(), key].map(str::to_owned).into())
}

#[cfg(test)]
mod tests;
