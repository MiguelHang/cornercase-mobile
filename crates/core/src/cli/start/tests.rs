use rstest::rstest;

use super::*;

fn agent(model: &str, worktree: &str) -> StartAgent {
    StartAgent {
        project: 3,
        agent: "claude".to_owned(),
        model: model.to_owned(),
        prompt: "fix the login".to_owned(),
        worktree: worktree.to_owned(),
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
