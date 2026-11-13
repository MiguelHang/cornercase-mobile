use rstest::rstest;
use serde_json::json;

use super::*;

const CHECK_SUBAGENT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/control/close-agent/0.13.31/check-subagent.json"
));
const CHECK_FLOW_MEMBER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/control/close-agent/0.13.31/check-flow-member.json"
));
const CLOSED: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/close-agent/0.13.31/closed.json"));

fn parsed(text: &str) -> AgentClosing {
    serde_json::from_str(text).expect("an agent closing")
}

fn claude(pane: u64, tab: u64, workspace: u64) -> PaneRef {
    PaneRef { agent: Some("claude".to_owned()), pane, project: 9, tab, workspace }
}

fn root_with_a_subagent(closed: bool) -> AgentClosing {
    AgentClosing {
        pane: 10,
        agent: "claude".to_owned(),
        project: 9,
        workspace: 8,
        tabs: vec![11, 13],
        stops: vec![claude(10, 11, 8), claude(12, 13, 8)],
        keeps: vec![claude(17, 18, 16)],
        run: None,
        closed,
        warning: None,
    }
}

mod argv {
    use super::*;

    const FORBIDDEN: [&str; 4] = ["--remove-worktree", "--delete-branch", "--force", "--unlock"];

    #[test]
    fn checks_one_agent_as_json_against_one_boot() {
        let argv = ["close", "--agent", "--pane", "10", "--boot", "4242-1", "--check", "--json"];
        assert_eq!(agent_args(10, "4242-1", true), argv);
    }

    #[test]
    fn closes_one_agent_as_json_against_one_boot() {
        assert_eq!(agent_args(10, "4242-1", false), ["close", "--agent", "--pane", "10", "--boot", "4242-1", "--json"]);
    }

    #[test]
    fn writes_the_whole_pane_number() {
        assert_eq!(agent_args(u64::MAX, "b", false)[3], u64::MAX.to_string());
    }

    #[rstest]
    #[case::check(true)]
    #[case::close(false)]
    fn always_names_the_boot_right_after_its_flag(#[case] check: bool) {
        let argv = agent_args(7, "9-3", check);
        let at = argv.iter().position(|word| word == "--boot").expect("a --boot");
        assert_eq!(argv[at + 1], "9-3");
    }

    #[rstest]
    #[case::check(true)]
    #[case::close(false)]
    fn never_removes_a_worktree_or_a_branch_nor_forces(#[case] check: bool) {
        let argv = agent_args(7, "9-3", check);
        let found: Vec<&String> = argv.iter().filter(|word| FORBIDDEN.contains(&word.as_str())).collect();
        assert!(found.is_empty(), "forbidden flags: {found:?}");
    }
}

mod reading {
    use super::*;

    #[test]
    fn reads_the_check_of_a_root_with_a_subagent_from_cornercase_0_13_31() {
        assert_eq!(parsed(CHECK_SUBAGENT), root_with_a_subagent(false));
    }

    #[test]
    fn reads_the_answer_of_a_close_from_cornercase_0_13_31() {
        assert_eq!(parsed(CLOSED), root_with_a_subagent(true));
    }

    #[test]
    fn reads_the_check_of_a_flow_member_from_cornercase_0_13_31() {
        let closing = parsed(CHECK_FLOW_MEMBER);
        let orchestrator = PaneRef { agent: None, pane: 21, project: 9, tab: 24, workspace: 19 };
        assert_eq!((closing.pane, closing.run, closing.closed), (22, Some(20), false));
        assert_eq!(closing.tabs, [24, 25, 26]);
        assert_eq!(closing.stops, [orchestrator, claude(22, 25, 19), claude(23, 26, 19)]);
        assert!(closing.keeps.is_empty(), "keeps: {:?}", closing.keeps);
    }

    #[test]
    fn ignores_keys_it_does_not_know() {
        let line = json!({
            "pane": 4, "agent": "codex", "closed": true,
            "removes": ["/Users/mara/wt"], "branches": ["feat/x"], "model": { "name": "gpt" },
            "stops": [{ "pane": 4, "tab": 5, "title": "Builder" }]
        });
        let closing = AgentClosing::deserialize(line).expect("an agent closing");
        let stop = PaneRef { pane: 4, tab: 5, ..PaneRef::default() };
        let expected = AgentClosing {
            pane: 4,
            agent: "codex".to_owned(),
            closed: true,
            stops: vec![stop],
            ..AgentClosing::default()
        };
        assert_eq!(closing, expected);
    }

    #[test]
    fn leaves_what_is_missing_empty() {
        assert_eq!(parsed(r#"{"pane": 4}"#), AgentClosing { pane: 4, ..AgentClosing::default() });
    }

    #[test]
    fn takes_a_null_run_warning_and_agent() {
        let closing = parsed(r#"{"pane": 4, "run": null, "warning": null, "stops": [{"pane": 4, "agent": null}]}"#);
        let stop = PaneRef { pane: 4, ..PaneRef::default() };
        assert_eq!(closing, AgentClosing { pane: 4, stops: vec![stop], ..AgentClosing::default() });
    }

    #[test]
    fn keeps_the_warning_it_is_given() {
        let closing = parsed(r#"{"pane": 4, "warning": "the flow run ends"}"#);
        assert_eq!(closing.warning.as_deref(), Some("the flow run ends"));
    }
}
