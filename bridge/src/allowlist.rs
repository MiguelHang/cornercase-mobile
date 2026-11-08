use cornercase_core::protocol::Refusal;

const REMOTE_FLAGS: &[&str] = &["--remote", "--remote-command"];
const BARE_FLAGS: &[&str] = &["--help", "-h", "--version", "-V"];

pub fn allowed(argv: &[String]) -> Result<(), Refusal> {
    refuse_remote(argv)?;
    let (first, rest) = refuse_empty(argv)?;
    if first.starts_with('-') {
        return bare_flag(first, rest);
    }
    Err(not_allowed(first))
}

fn refuse_remote(argv: &[String]) -> Result<(), Refusal> {
    match argv.iter().find_map(|arg| remote_flag(arg)) {
        Some(flag) => {
            Err(refused(&format!("{flag} is not allowed through the bridge: the bridge runs commands on this Mac")))
        }
        None => Ok(()),
    }
}

fn remote_flag(arg: &str) -> Option<&'static str> {
    REMOTE_FLAGS
        .iter()
        .copied()
        .find(|flag| arg.strip_prefix(flag).is_some_and(|rest| rest.is_empty() || rest.starts_with('=')))
}

fn refuse_empty(argv: &[String]) -> Result<(&str, &[String]), Refusal> {
    argv.split_first()
        .map(|(first, rest)| (first.as_str(), rest))
        .ok_or_else(|| refused("no command: the bridge does not open the window"))
}

fn bare_flag(flag: &str, rest: &[String]) -> Result<(), Refusal> {
    match (BARE_FLAGS.contains(&flag), rest.is_empty()) {
        (true, true) => Ok(()),
        (true, false) => Err(refused(&format!("`{flag}` is only allowed alone through the bridge"))),
        (false, _) => Err(not_allowed(flag)),
    }
}

fn not_allowed(name: &str) -> Refusal {
    refused(&format!("`{name}` is not allowed through the bridge"))
}

fn refused(message: &str) -> Refusal {
    Refusal { message: message.to_owned() }
}

#[cfg(test)]
mod tests;
