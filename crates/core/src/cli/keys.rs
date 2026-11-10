use crate::error::Error;

const NAMES: [&str; 18] =
    ["esc", "tab", "up", "down", "left", "right", "enter", "ctrl+c", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

pub fn args(pane: u64, keys: &[&str]) -> Result<Vec<String>, Error> {
    if let Some(key) = keys.iter().copied().find(|key| !NAMES.contains(key)) {
        return Err(Error::UnknownKey(key.to_owned()));
    }
    if keys.is_empty() {
        return Err(Error::UnknownKey(String::new()));
    }
    let pane = pane.to_string();
    Ok(["keys", "--pane", &pane].into_iter().chain(keys.iter().copied()).map(str::to_owned).collect())
}

#[cfg(test)]
mod tests;
