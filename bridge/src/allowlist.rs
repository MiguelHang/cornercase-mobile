use cornercase_core::protocol::Refusal;

const ECHOED_CHARS: usize = 80;
const REMOTE_FLAGS: &[&str] = &["--remote", "--remote-command"];
const BARE_FLAGS: &[&str] = &["--help", "-h", "--version", "-V"];
const CONTROL: &[&str] = &[
    "status",
    "open",
    "clone",
    "new-workspace",
    "new-tab",
    "split",
    "setup",
    "start",
    "send",
    "keys",
    "held",
    "read",
    "wait",
    "close",
    "rename",
    "focus",
    "go",
    "notify",
    "away",
    "mark",
    "search",
    "todo",
    "queue",
    "events",
    "flow",
    "contract",
    "surface",
    "view",
    "view-set",
    "input",
    "requests",
    "scroll",
    "selection",
    "resize",
    "redraw",
    "ratio",
    "link",
    "paste-file",
    "theme",
    "config",
    "move",
    "move-pane",
    "collapse",
    "expand",
    "group",
    "issue",
    "changes",
    "files",
    "usage",
    "ports",
    "pull-request",
    "context",
    "recap",
    "processes",
    "worktrees",
    "machines",
    "tour",
    "agents",
    "account",
    "profile",
];
const OTHERS: &[&str] = &["stats"];
const PLUGIN: &str = "plugin";
const PLUGIN_ACTIONS: &[&str] = &["actions", "status", "run", "popup-close"];

pub fn allowed(argv: &[String]) -> Result<(), Refusal> {
    refuse_remote(argv)?;
    let (first, rest) = refuse_empty(argv)?;
    match first {
        flag if flag.starts_with('-') => bare_flag(flag, rest),
        PLUGIN => plugin_action(rest),
        name if CONTROL.contains(&name) || OTHERS.contains(&name) => Ok(()),
        name => Err(not_allowed(name)),
    }
}

fn refuse_remote(argv: &[String]) -> Result<(), Refusal> {
    match argv.iter().find_map(|arg| remote_flag(arg)) {
        Some(flag) => Err(Refusal::new(&format!(
            "{flag} is not allowed through the bridge: the bridge runs commands on this Mac"
        ))),
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
        .ok_or_else(|| Refusal::new("no command: the bridge does not open the window"))
}

fn bare_flag(flag: &str, rest: &[String]) -> Result<(), Refusal> {
    match (BARE_FLAGS.contains(&flag), rest.is_empty()) {
        (true, true) => Ok(()),
        (true, false) => Err(Refusal::new(&format!("`{flag}` is only allowed alone through the bridge"))),
        (false, _) => Err(not_allowed(flag)),
    }
}

fn plugin_action(rest: &[String]) -> Result<(), Refusal> {
    match rest.first() {
        Some(action) if PLUGIN_ACTIONS.contains(&action.as_str()) => Ok(()),
        Some(action) => Err(not_allowed(&format!("{PLUGIN} {action}"))),
        None => Err(not_allowed(PLUGIN)),
    }
}

fn not_allowed(name: &str) -> Refusal {
    Refusal::new(&format!("`{}` is not allowed through the bridge", echoed(name)))
}

fn echoed(name: &str) -> String {
    match name.char_indices().nth(ECHOED_CHARS) {
        Some((end, _)) => format!("{}…", &name[..end]),
        None => name.to_owned(),
    }
}

#[cfg(test)]
mod tests;
