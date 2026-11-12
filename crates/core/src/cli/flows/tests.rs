use rstest::rstest;
use serde_json::json;

use super::*;
use crate::cli::flows::fixtures::{BUILTINS, NO_ORCHESTRATOR, PROJECT, RUN};

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

const ISSUE_BRANCH: &str = "issue-482-login-form-forgets-the-email";

fn run(branch: &str) -> StartFlow {
    StartFlow {
        project: 8,
        name: "Quick".to_owned(),
        key: "#482".to_owned(),
        source: "github".to_owned(),
        branch: branch.to_owned(),
        issue_branch: ISSUE_BRANCH.to_owned(),
        accounts: vec![],
    }
}

fn started(project: u64, workspace: u64, tab: u64, pane: u64) -> Started {
    Started { project, workspace, tab, pane, warning: None }
}

mod start_flow {
    use super::*;

    const HEAD: [&str; 6] = ["flow", "run", "--project", "8", "--issue=#482", "--source=github"];

    fn around(middle: &[&str], name: &str) -> Vec<String> {
        HEAD.iter().chain(middle).chain(&["--json", "--", name]).map(|&word| word.to_owned()).collect()
    }

    #[rstest]
    #[case::the_issue_s_branch(run(ISSUE_BRANCH), &[])]
    #[case::the_issue_s_branch_with_spaces(run(" issue-482-login-form-forgets-the-email\n"), &[])]
    #[case::another_branch(run("fix/login-email"), &["--worktree=fix/login-email"])]
    #[case::no_branch(run(""), &[])]
    #[case::a_branch_of_only_spaces(run(" \t"), &[])]
    fn names_a_branch_only_when_it_is_not_the_issue_s(#[case] form: StartFlow, #[case] middle: &[&str]) {
        assert_eq!(form.argv(), around(middle, "Quick"));
    }

    #[test]
    fn keeps_a_name_with_spaces_in_one_word_after_the_options() {
        let form = StartFlow { name: "Full issue".to_owned(), ..run("") };
        assert_eq!(form.argv(), around(&[], "Full issue"));
    }

    #[test]
    fn glues_a_key_and_a_branch_that_start_with_a_dash_and_ends_the_options_before_the_name() {
        let form = StartFlow { key: "-48".to_owned(), name: "-weird".to_owned(), ..run("-b") };
        let expected = [
            "flow",
            "run",
            "--project",
            "8",
            "--issue=-48",
            "--source=github",
            "--worktree=-b",
            "--json",
            "--",
            "-weird",
        ];
        assert_eq!(form.argv(), expected);
    }

    #[test]
    fn trims_every_value() {
        let form = StartFlow {
            project: 8,
            name: " Bug, test first\n".to_owned(),
            key: "  #482 ".to_owned(),
            source: " github\t".to_owned(),
            branch: " fix/login-email ".to_owned(),
            issue_branch: ISSUE_BRANCH.to_owned(),
            accounts: vec![],
        };
        assert_eq!(form.argv(), around(&["--worktree=fix/login-email"], "Bug, test first"));
    }

    #[test]
    fn leaves_out_a_blank_tracker() {
        let form = StartFlow { source: " ".to_owned(), ..run("") };
        assert_eq!(form.argv(), ["flow", "run", "--project", "8", "--issue=#482", "--json", "--", "Quick"]);
    }

    #[test]
    fn names_each_chosen_account_in_the_order_given_before_the_name() {
        let accounts = [("codex", "work"), ("claude", "")]
            .map(|(agent, account)| AccountPick { agent: agent.to_owned(), account: account.to_owned() });
        let form = StartFlow { accounts: accounts.into(), ..run("fix/login-email") };
        let middle = ["--worktree=fix/login-email", "--account=codex=work", "--account=claude="];
        assert_eq!(form.argv(), around(&middle, "Quick"));
    }
}

mod a_started_run {
    use super::*;

    fn member(role: &str, agent: &str, tab: u64, pane: u64) -> serde_json::Value {
        json!({ "role": role, "agent": agent, "tab": tab, "pane": pane })
    }

    #[test]
    fn opens_the_orchestrator_s_pane_with_cornercase_s_note() {
        let note = "this project's flow passes --search to `coder`; they run without asking, so check them";
        let expected = FlowStarted {
            started: Started { warning: Some(note.to_owned()), ..started(8, 9, 14, 11) },
            agent: "claude".to_owned(),
        };
        assert_eq!(flow_started(RUN), Ok(expected));
    }

    #[test]
    fn opens_the_first_member_s_pane_when_no_one_orchestrates() {
        let expected = FlowStarted { started: started(8, 17, 23, 20), agent: "claude".to_owned() };
        assert_eq!(flow_started(NO_ORCHESTRATOR), Ok(expected));
    }

    #[test]
    fn finds_the_orchestrator_wherever_it_is_listed() {
        let members = [member("coder", "claude", 15, 12), member("orchestrator", "codex", 14, 11)];
        let text = json!({ "project": 8, "workspace": 9, "members": members, "extra": true }).to_string();
        let expected = FlowStarted { started: started(8, 9, 14, 11), agent: "codex".to_owned() };
        assert_eq!(flow_started(&text), Ok(expected));
    }

    #[rstest]
    #[case::no_members(json!({ "project": 8, "workspace": 9, "members": [] }).to_string())]
    #[case::nothing(json!({}).to_string())]
    fn says_a_run_without_agents_is_not_an_answer(#[case] text: String) {
        assert_eq!(flow_started(&text), Err(Error::Json(NO_MEMBERS.to_owned())));
    }

    #[test]
    fn says_why_the_text_is_not_json() {
        assert_eq!(flow_started("run 10"), Err(Error::Json("not JSON at line 1 column 1".to_owned())));
    }
}
