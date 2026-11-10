use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::cli::start::fixtures::{NEW_WORKSPACE, START};

fn agent(model: &str, worktree: &str) -> StartAgent {
    StartAgent {
        project: 3,
        agent: "claude".to_owned(),
        model: model.to_owned(),
        prompt: "fix the login".to_owned(),
        worktree: worktree.to_owned(),
    }
}

fn issue(model: &str, branch: &str) -> StartIssue {
    StartIssue {
        project: 3,
        agent: "claude".to_owned(),
        model: model.to_owned(),
        key: "#41".to_owned(),
        source: "github".to_owned(),
        branch: branch.to_owned(),
        issue_branch: "issue-41-checkout".to_owned(),
    }
}

fn workspace(name: &str, worktree: bool) -> NewWorkspace {
    NewWorkspace { project: 7, name: name.to_owned(), worktree }
}

mod start {
    use super::*;

    #[rstest]
    #[case::neither(agent("", ""), &["start", "claude", "--project", "3", "--prompt=fix the login", "--placed", "--json"])]
    #[case::model(agent("opus", ""), &["start", "claude", "--project", "3", "--model=opus", "--prompt=fix the login", "--placed", "--json"])]
    #[case::worktree(agent("", "fix/login"), &["start", "claude", "--project", "3", "--worktree=fix/login", "--prompt=fix the login", "--placed", "--json"])]
    #[case::both(agent("opus", "fix/login"), &["start", "claude", "--project", "3", "--worktree=fix/login", "--model=opus", "--prompt=fix the login", "--placed", "--json"])]
    fn sends_the_model_and_the_branch_only_when_given(#[case] form: StartAgent, #[case] expected: &[&str]) {
        assert_eq!(form.argv(), expected);
    }

    #[test]
    fn keeps_a_prompt_of_many_lines_in_one_word() {
        let form = StartAgent { prompt: "first\n\n  second\nthird".to_owned(), ..agent("", "") };
        assert_eq!(form.argv()[4], "--prompt=first\n\n  second\nthird");
    }

    #[test]
    fn glues_values_that_start_with_a_dash_to_their_option() {
        let form = StartAgent { prompt: "--help".to_owned(), ..agent("-m", "-b") };
        assert_eq!(form.argv()[4..7], ["--worktree=-b", "--model=-m", "--prompt=--help"]);
    }

    #[test]
    fn trims_every_value() {
        let form = StartAgent {
            project: 3,
            agent: " codex\n".to_owned(),
            model: "  gpt-5.5 ".to_owned(),
            prompt: "\n  fix it \n".to_owned(),
            worktree: " fix/it ".to_owned(),
        };
        let expected = [
            "start",
            "codex",
            "--project",
            "3",
            "--worktree=fix/it",
            "--model=gpt-5.5",
            "--prompt=fix it",
            "--placed",
            "--json",
        ];
        assert_eq!(form.argv(), expected);
    }

    #[test]
    fn leaves_out_a_model_and_a_branch_of_only_spaces() {
        assert_eq!(agent(" \t", "\n ").argv(), agent("", "").argv());
    }

    #[test]
    fn never_moves_the_window_of_the_mac() {
        assert!(!agent("opus", "fix/login").argv().iter().any(|word| word == "--focus"));
    }
}

mod start_issue {
    use super::*;

    const HEAD: [&str; 6] = ["start", "claude", "--project", "3", "--issue=#41", "--source=github"];

    fn after_the_head(rest: &[&str]) -> Vec<String> {
        HEAD.iter().chain(rest).map(|&word| word.to_owned()).collect()
    }

    #[rstest]
    #[case::the_issue_s_branch(issue("", "issue-41-checkout"), &["--placed", "--json"])]
    #[case::the_issue_s_branch_and_a_model(issue("opus", "issue-41-checkout"), &["--model=opus", "--placed", "--json"])]
    #[case::another_branch(issue("", "fix/coupon"), &["--worktree=fix/coupon", "--placed", "--json"])]
    #[case::another_branch_and_a_model(
        issue("opus", "fix/coupon"),
        &["--worktree=fix/coupon", "--model=opus", "--placed", "--json"]
    )]
    #[case::no_branch(issue("", ""), &["--placed", "--json"])]
    #[case::the_issue_s_branch_with_spaces(issue(" \t", " issue-41-checkout\n"), &["--placed", "--json"])]
    fn names_a_branch_only_when_it_is_not_the_issue_s(#[case] form: StartIssue, #[case] rest: &[&str]) {
        assert_eq!(form.argv(), after_the_head(rest));
    }

    #[test]
    fn leaves_out_a_blank_tracker() {
        let form = StartIssue { source: " ".to_owned(), ..issue("", "") };
        assert_eq!(form.argv(), ["start", "claude", "--project", "3", "--issue=#41", "--placed", "--json"]);
    }

    #[test]
    fn glues_a_key_and_a_branch_that_start_with_a_dash_to_their_option() {
        let form = StartIssue { key: " -48 ".to_owned(), source: "shortcut".to_owned(), ..issue("-m", "-b") };
        assert_eq!(form.argv()[4..8], ["--issue=-48", "--source=shortcut", "--worktree=-b", "--model=-m"]);
    }

    #[test]
    fn never_sends_a_prompt_nor_moves_the_window_of_the_mac() {
        let argv = issue("opus", "fix/coupon").argv();
        assert!(!argv.iter().any(|word| word.starts_with("--prompt") || word == "--focus"), "{argv:?}");
    }
}

mod new_workspace {
    use super::*;

    #[rstest]
    #[case::plain(workspace("experiments", false), &["new-workspace", "--project", "7", "--json", "--", "experiments"])]
    #[case::worktree(workspace("fix/login", true), &["new-workspace", "--project", "7", "--worktree", "--json", "--", "fix/login"])]
    #[case::dash(workspace("-rf", false), &["new-workspace", "--project", "7", "--json", "--", "-rf"])]
    #[case::spaces(workspace("  experiments \n", true), &["new-workspace", "--project", "7", "--worktree", "--json", "--", "experiments"])]
    fn names_the_workspace_after_a_double_dash(#[case] form: NewWorkspace, #[case] expected: &[&str]) {
        assert_eq!(form.argv(), expected);
    }
}

mod started {
    use super::*;

    fn ids(project: u64, workspace: u64, tab: u64, pane: u64) -> Started {
        Started { project, workspace, tab, pane, warning: None }
    }

    fn with_warning(text: &str) -> String {
        let mut value: Value = serde_json::from_str(text).expect("the fixture is JSON");
        value["warning"] = json!("the setup script failed");
        value.to_string()
    }

    #[rstest]
    #[case::start(START, ids(3, 12, 40, 57))]
    #[case::new_workspace(NEW_WORKSPACE, ids(7, 13, 41, 58))]
    fn reads_the_ids_cornercase_0_13_19_answers(#[case] text: &str, #[case] expected: Started) {
        assert_eq!(started(text), Ok(expected));
    }

    #[rstest]
    #[case::start(START, ids(3, 12, 40, 57))]
    #[case::new_workspace(NEW_WORKSPACE, ids(7, 13, 41, 58))]
    fn keeps_the_warning_when_there_is_one(#[case] text: &str, #[case] ids: Started) {
        let expected = Started { warning: Some("the setup script failed".to_owned()), ..ids };
        assert_eq!(started(&with_warning(text)), Ok(expected));
    }

    #[test]
    fn ignores_keys_it_does_not_know_and_leaves_what_is_missing_empty() {
        let text = json!({ "pane": 57, "projection": 2 }).to_string();
        assert_eq!(started(&text), Ok(Started { pane: 57, ..Started::default() }));
    }

    #[rstest]
    #[case::not_json("pane 57", "not JSON at line 1 column 1")]
    #[case::cut_short("{\"pane\": 57", "the JSON ends too early at line 1 column 11")]
    fn says_why_the_text_is_not_an_answer(#[case] text: &str, #[case] reason: &str) {
        assert_eq!(started(text), Err(Error::Json(reason.to_owned())));
    }

    #[test]
    fn says_a_value_has_the_wrong_shape_but_not_what_it_says() {
        let error = started("{\"warning\": [\"secret-token\"]}").expect_err("a wrong shape");
        let Error::Json(reason) = error else { panic!("not a JSON error: {error:?}") };
        assert!(reason.starts_with("not the expected shape at line 1 column"), "{reason}");
        assert!(!reason.contains("secret-token"), "{reason}");
    }
}
