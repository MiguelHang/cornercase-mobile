use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StartAgent {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    pub agent: String,
    pub model: String,
    pub prompt: String,
    pub worktree: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NewWorkspace {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub project: u64,
    pub name: String,
    pub worktree: bool,
}

impl StartAgent {
    pub fn argv(&self) -> Vec<String> {
        let mut argv = words(&["start", self.agent.trim(), "--project", &self.project.to_string()]);
        argv.extend(optional("--worktree", &self.worktree));
        argv.extend(optional("--model", &self.model));
        argv.push(format!("--prompt={}", self.prompt.trim()));
        argv.extend(words(&["--placed", "--json"]));
        argv
    }
}

impl NewWorkspace {
    pub fn argv(&self) -> Vec<String> {
        let mut argv = words(&["new-workspace", "--project", &self.project.to_string()]);
        argv.extend(self.worktree.then(|| "--worktree".to_owned()));
        argv.extend(words(&["--json", "--", self.name.trim()]));
        argv
    }
}

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|&word| word.to_owned()).collect()
}

fn optional(name: &str, value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| format!("{name}={value}"))
}

#[cfg(test)]
mod tests;
