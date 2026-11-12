use std::iter::once;

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::transport::json_error;

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct AgentChoice {
    pub agent: String,
    pub command: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum Login {
    SignedIn,
    SignedOut,
    Failed,
    #[default]
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AccountChoice {
    pub name: String,
    pub label: String,
    pub login: Login,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AgentAccounts {
    pub agent: String,
    pub default: String,
    pub options: Vec<AccountChoice>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StartAgents {
    pub agents: Vec<AgentChoice>,
    pub accounts: Vec<AgentAccounts>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Agents {
    agents: Vec<AgentChoice>,
    providers: Vec<AgentLogins>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AgentLogins {
    provider: String,
    accounts: Vec<Account>,
    system_label: Option<String>,
    system_login: Checked,
    active: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Account {
    name: String,
    label: Option<String>,
    login: Checked,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Checked {
    state: Login,
}

const SYSTEM: &str = "System";

impl Agents {
    fn installed(self) -> StartAgents {
        let agents = self.agents.into_iter().filter(|choice| choice.installed).collect();
        let with_accounts = self.providers.into_iter().filter(|provider| !provider.accounts.is_empty());
        StartAgents { agents, accounts: with_accounts.map(AgentAccounts::from).collect() }
    }
}

impl From<AgentLogins> for AgentAccounts {
    fn from(provider: AgentLogins) -> Self {
        let active = provider.active.unwrap_or_default();
        let known = provider.accounts.iter().any(|account| account.name == active);
        let label = provider.system_label.unwrap_or_else(|| SYSTEM.to_owned());
        let own = AccountChoice { name: String::new(), label, login: provider.system_login.state };
        let options = once(own).chain(provider.accounts.into_iter().map(AccountChoice::from)).collect();
        Self { agent: provider.provider, default: if known { active } else { String::new() }, options }
    }
}

impl From<Account> for AccountChoice {
    fn from(account: Account) -> Self {
        let label = account.label.unwrap_or_else(|| account.name.clone());
        Self { name: account.name, label, login: account.login.state }
    }
}

pub fn installed(text: &str) -> Result<StartAgents, Error> {
    let agents: Agents = serde_json::from_str(text).map_err(|error| json_error(&error))?;
    Ok(agents.installed())
}

#[cfg(test)]
mod tests;
