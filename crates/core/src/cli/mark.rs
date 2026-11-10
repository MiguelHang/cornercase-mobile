pub fn reviewed_args(pane: u64) -> Vec<String> {
    ["mark", "--pane", &pane.to_string(), "--reviewed"].map(str::to_owned).into()
}

#[cfg(test)]
mod tests;
