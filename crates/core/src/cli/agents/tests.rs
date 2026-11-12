use rstest::rstest;
use serde_json::json;

use super::*;
use crate::cli::agents::fixtures::{ACCOUNT_LIST, ACCOUNTS, SIGNED_OUT};

fn choice(agent: &str, command: &str, installed: bool) -> AgentChoice {
    AgentChoice { agent: agent.to_owned(), command: command.to_owned(), installed }
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
        assert_eq!(installed(ACCOUNT_LIST), Ok(vec![choice("claude", "claude", true)]));
    }

    #[rstest]
    #[case::signed_in(ACCOUNTS)]
    #[case::signed_out(SIGNED_OUT)]
    fn keeps_only_the_installed_agents_of_cornercase_0_13_31_with_accounts(#[case] text: &str) {
        assert_eq!(installed(text), Ok(vec![choice("claude", "claude", true)]));
    }

    #[test]
    fn ignores_keys_it_does_not_know() {
        let text = json!({
            "profile": "work",
            "providers": [{ "email": "me@example.com" }],
            "agents": [{ "agent": "codex", "command": "codex", "installed": true, "path": "/usr/local/bin/codex" }],
        });
        assert_eq!(installed(&text.to_string()), Ok(vec![choice("codex", "codex", true)]));
    }

    #[rstest]
    #[case::no_agents(json!({ "providers": [] }))]
    #[case::an_empty_list(json!({ "agents": [] }))]
    #[case::none_installed(json!({ "agents": [{ "agent": "claude", "command": "claude", "installed": false }] }))]
    fn has_no_choice_without_installed_agents(#[case] value: serde_json::Value) {
        assert_eq!(installed(&value.to_string()), Ok(vec![]));
    }

    #[test]
    fn takes_an_agent_without_installed_as_not_installed() {
        assert_eq!(installed(&json!({ "agents": [{ "agent": "claude" }] }).to_string()), Ok(vec![]));
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
        let choices = installed(&text.to_string()).expect("a list");
        assert_eq!(names(&choices), ["opencode", "codex", "aider"]);
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
