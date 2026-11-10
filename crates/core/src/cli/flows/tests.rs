use serde_json::json;

use super::*;
use crate::cli::flows::fixtures::{BUILTINS, PROJECT};

fn flows(text: &str) -> Flows {
    serde_json::from_str(text).expect("a list of flows")
}

fn names(read: &Flows) -> Vec<&str> {
    read.flows.iter().map(|flow| flow.name.as_str()).collect()
}

fn team(flow: &FlowChoice) -> Vec<(&str, &str)> {
    flow.roles.iter().map(|role| (role.role.as_str(), role.agent.as_str())).collect()
}

fn named<'a>(read: &'a Flows, name: &str) -> &'a FlowChoice {
    read.flows.iter().find(|flow| flow.name == name).expect("the flow by its name")
}

mod the_arguments {
    use super::*;

    #[test]
    fn ask_for_a_project_s_flows() {
        assert_eq!(list_args(8), ["flow", "list", "--project", "8", "--json"]);
    }
}

mod the_built_ins {
    use super::*;

    #[test]
    fn are_read_in_cornercase_s_order() {
        assert_eq!(names(&flows(BUILTINS)), ["Arena", "Bug, test first", "Cross review", "Full issue", "Quick"]);
    }

    #[test]
    fn read_each_role_with_its_agent_the_orchestrator_first() {
        let read = flows(BUILTINS);
        let expected = [("orchestrator", "claude"), ("coder", "claude"), ("reviewer", "codex")];
        assert_eq!(team(named(&read, "Quick")), expected);
    }

    #[test]
    fn read_with_their_description_none_by_default_and_none_with_problems() {
        let read = flows(BUILTINS);
        let quick = named(&read, "Quick");
        assert!(quick.description.starts_with("A coder and one round of review"), "{quick:?}");
        assert!(read.flows.iter().all(|flow| !flow.default && flow.problems.is_empty()), "{read:?}");
    }
}

mod a_project_s_flows {
    use super::*;

    #[test]
    fn are_read_in_the_user_s_order_without_the_broken_and_hidden_ones() {
        let expected = ["Ship it", "Quick", "Bug, test first", "Cross review", "Full issue", "Second opinion"];
        assert_eq!(names(&flows(PROJECT)), expected);
    }

    #[test]
    fn read_which_one_is_the_project_s_default() {
        let read = flows(PROJECT);
        let defaults: Vec<_> = read.flows.iter().filter(|flow| flow.default).map(|flow| flow.name.as_str()).collect();
        assert_eq!(defaults, ["Ship it"]);
        assert_eq!(team(&read.flows[0]), [("orchestrator", "claude"), ("coder", "codex"), ("tester", "claude")]);
    }

    #[test]
    fn read_why_a_flow_cannot_run() {
        let read = flows(PROJECT);
        let second = named(&read, "Second opinion");
        assert_eq!(second.problems, ["`summary` is in no stage"]);
        assert!(!second.default);
    }
}

mod leniency {
    use super::*;

    #[test]
    fn ignores_the_fields_it_does_not_use() {
        let line = json!({
            "flows": [{
                "name": "Quick", "origin": "project", "path": "/p/.cornercase/flows/quick.toml", "edited": true,
                "check": "cargo test", "flow": { "stages": [] },
                "roles": [{ "role": "coder", "agent": "codex", "edits": true, "model": "o3", "worktree": "own" }]
            }],
            "broken": [{ "path": "/p/draft.toml", "error": "line 3" }],
            "hidden": [{ "name": "Arena" }],
            "builtins": ["Arena", "Quick"]
        });
        let role = FlowRole { role: "coder".to_owned(), agent: "codex".to_owned() };
        let expected = FlowChoice { name: "Quick".to_owned(), roles: vec![role], ..FlowChoice::default() };
        assert_eq!(flows(&line.to_string()).flows, [expected]);
    }

    #[test]
    fn reads_missing_fields_as_their_defaults() {
        assert_eq!(flows("{}"), Flows::default());
        let bare = flows(&json!({ "flows": [{ "roles": [{}] }] }).to_string());
        let expected = FlowChoice { roles: vec![FlowRole::default()], ..FlowChoice::default() };
        assert_eq!(bare.flows, [expected]);
    }
}
