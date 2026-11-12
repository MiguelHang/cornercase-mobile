use rstest::rstest;
use serde_json::json;

use super::*;
use crate::cli::agents::fixtures::{ACCOUNT_LIST, ACCOUNTS, SIGNED_OUT};

fn choice(agent: &str, command: &str, installed: bool) -> AgentChoice {
    AgentChoice { agent: agent.to_owned(), command: command.to_owned(), installed }
}

fn agents(text: &str) -> Vec<AgentChoice> {
    installed(text).expect("a list").agents
}

fn accounts(value: &serde_json::Value) -> Vec<AgentAccounts> {
    installed(&value.to_string()).expect("a list").accounts
}

fn option(name: &str, label: &str, login: Login) -> AccountChoice {
    AccountChoice { name: name.to_owned(), label: label.to_owned(), login }
}

fn claude(default: &str, options: Vec<AccountChoice>) -> AgentAccounts {
    AgentAccounts { agent: "claude".to_owned(), default: default.to_owned(), options }
}

fn names(choices: &[AgentChoice]) -> Vec<&str> {
    choices.iter().map(|choice| choice.agent.as_str()).collect()
}

mod reading {
    use super::*;

    #[test]
    fn reads_every_agent_cornercase_0_13_19_knows() {
        let agents: Agents = serde_json::from_str(ACCOUNT_LIST).expect("the real list");
        assert_eq!(agents.agents.len(), 16);
        assert_eq!(agents.agents[0], choice("claude", "claude", true));
        assert_eq!(agents.agents[5], choice("cursor", "cursor-agent", false));
    }

    #[test]
    fn keeps_only_the_installed_agents_of_cornercase_0_13_19() {
        assert_eq!(agents(ACCOUNT_LIST), [choice("claude", "claude", true)]);
    }

    #[rstest]
    #[case::signed_in(ACCOUNTS)]
    #[case::signed_out(SIGNED_OUT)]
    fn keeps_only_the_installed_agents_of_cornercase_0_13_31_with_accounts(#[case] text: &str) {
        assert_eq!(agents(text), [choice("claude", "claude", true)]);
    }

    #[test]
    fn ignores_keys_it_does_not_know() {
        let text = json!({
            "profile": "work",
            "providers": [{ "email": "me@example.com" }],
            "agents": [{ "agent": "codex", "command": "codex", "installed": true, "path": "/usr/local/bin/codex" }],
        });
        assert_eq!(agents(&text.to_string()), [choice("codex", "codex", true)]);
    }

    #[rstest]
    #[case::no_agents(json!({ "providers": [] }))]
    #[case::an_empty_list(json!({ "agents": [] }))]
    #[case::none_installed(json!({ "agents": [{ "agent": "claude", "command": "claude", "installed": false }] }))]
    fn has_no_choice_without_installed_agents(#[case] value: serde_json::Value) {
        assert_eq!(agents(&value.to_string()), []);
    }

    #[test]
    fn takes_an_agent_without_installed_as_not_installed() {
        assert_eq!(agents(&json!({ "agents": [{ "agent": "claude" }] }).to_string()), []);
    }
}

mod accounts {
    use super::*;

    #[test]
    fn has_none_on_cornercase_0_13_19_without_accounts() {
        assert_eq!(installed(ACCOUNT_LIST).expect("the real list").accounts, []);
    }

    #[test]
    fn offers_the_own_login_by_its_label_then_each_account_of_cornercase_0_13_31() {
        let expected = vec![option("", "work", Login::SignedIn), option("personal", "personal", Login::SignedIn)];
        assert_eq!(installed(ACCOUNTS).expect("the real list").accounts, [claude("", expected)]);
    }

    #[test]
    fn picks_the_active_account_and_says_who_is_signed_out_on_cornercase_0_13_31() {
        let expected = vec![
            option("", "System", Login::SignedOut),
            option("home", "home", Login::SignedOut),
            option("work", "Work", Login::SignedOut),
        ];
        assert_eq!(installed(SIGNED_OUT).expect("the real list").accounts, [claude("work", expected)]);
    }

    #[rstest]
    #[case::no_providers(json!({ "agents": [] }))]
    #[case::an_empty_list(json!({ "providers": [] }))]
    #[case::no_accounts(json!({ "providers": [{ "provider": "claude", "active": "work" }] }))]
    #[case::empty_accounts(json!({ "providers": [{ "provider": "codex", "accounts": [] }] }))]
    fn leaves_out_agents_without_accounts(#[case] value: serde_json::Value) {
        assert_eq!(accounts(&value), []);
    }

    #[rstest]
    #[case::missing(json!({ "provider": "claude", "accounts": [{ "name": "home" }] }))]
    #[case::empty(json!({ "provider": "claude", "active": "", "accounts": [{ "name": "home" }] }))]
    #[case::unknown(json!({ "provider": "claude", "active": "gone", "accounts": [{ "name": "home" }] }))]
    #[case::null(json!({ "provider": "claude", "active": null, "accounts": [{ "name": "home" }] }))]
    fn defaults_to_the_own_login_without_a_known_active_account(#[case] provider: serde_json::Value) {
        let expected = vec![option("", "System", Login::Unknown), option("home", "home", Login::Unknown)];
        assert_eq!(accounts(&json!({ "providers": [provider] })), [claude("", expected)]);
    }

    #[rstest]
    #[case::signed_in("signed-in", Login::SignedIn)]
    #[case::signed_out("signed-out", Login::SignedOut)]
    #[case::failed("failed", Login::Failed)]
    #[case::unknown("unknown", Login::Unknown)]
    #[case::a_new_state("expired", Login::Unknown)]
    fn reads_the_state_of_each_login(#[case] state: &str, #[case] login: Login) {
        let provider = json!({
            "provider": "claude",
            "system_login": { "state": state },
            "accounts": [{ "name": "home", "login": { "state": state, "checked": "2026-11-12T08:20:12.626Z" } }],
        });
        let expected = vec![option("", "System", login), option("home", "home", login)];
        assert_eq!(accounts(&json!({ "providers": [provider] })), [claude("", expected)]);
    }

    #[test]
    fn keeps_the_order_and_the_agents_cornercase_gives() {
        let value = json!({ "providers": [
            { "provider": "codex", "accounts": [{ "name": "b" }, { "name": "a", "label": "A" }] },
            { "provider": "gemini", "accounts": [] },
            { "provider": "claude", "system_label": "me", "accounts": [{ "name": "c" }] },
        ] });
        let codex = AgentAccounts {
            agent: "codex".to_owned(),
            default: String::new(),
            options: vec![
                option("", "System", Login::Unknown),
                option("b", "b", Login::Unknown),
                option("a", "A", Login::Unknown),
            ],
        };
        let claude = claude("", vec![option("", "me", Login::Unknown), option("c", "c", Login::Unknown)]);
        assert_eq!(accounts(&value), [codex, claude]);
    }

    #[test]
    fn sends_the_login_states_the_way_cornercase_writes_them() {
        let value = serde_json::to_value(option("", "System", Login::SignedOut)).expect("serialised");
        assert_eq!(value, json!({ "name": "", "label": "System", "login": "signed-out" }));
    }
}

mod order {
    use super::*;

    #[test]
    fn keeps_the_order_cornercase_gives() {
        let text = json!({
            "agents": [
                { "agent": "opencode", "command": "opencode", "installed": true },
                { "agent": "claude", "command": "claude", "installed": false },
                { "agent": "codex", "command": "codex", "installed": true },
                { "agent": "aider", "command": "aider", "installed": true },
            ],
        });
        assert_eq!(names(&agents(&text.to_string())), ["opencode", "codex", "aider"]);
    }
}

mod shape {
    use super::*;

    #[rstest]
    #[case::not_json("agents", "not JSON at line 1 column 1")]
    #[case::cut_short("{\"agents\": [", "the JSON ends too early at line 1 column 12")]
    fn says_why_the_text_is_not_a_list(#[case] text: &str, #[case] reason: &str) {
        assert_eq!(installed(text), Err(Error::Json(reason.to_owned())));
    }

    #[test]
    fn says_where_the_text_has_the_wrong_shape_but_not_what_it_says() {
        let error = installed("{\"agents\": \"secret-token\"}").expect_err("a wrong shape");
        let Error::Json(reason) = error else { panic!("not a JSON error: {error:?}") };
        assert!(reason.starts_with("not the expected shape at line 1"), "{reason}");
        assert!(!reason.contains("secret-token"), "{reason}");
    }
}
