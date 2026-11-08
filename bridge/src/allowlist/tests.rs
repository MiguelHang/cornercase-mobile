use rstest::rstest;

use super::*;

fn argv(line: &str) -> Vec<String> {
    line.split(' ').filter(|word| !word.is_empty()).map(str::to_owned).collect()
}

fn refusal_of(argv: &[String]) -> String {
    allowed(argv).expect_err("the bridge refuses it").message
}

mod remote {
    use super::*;

    const REMOTE: &str = "--remote is not allowed through the bridge: the bridge runs commands on this Mac";
    const REMOTE_COMMAND: &str =
        "--remote-command is not allowed through the bridge: the bridge runs commands on this Mac";

    #[rstest]
    #[case::first("--remote mac status")]
    #[case::first_with_value("--remote=mac status")]
    #[case::middle("status --remote mac --json")]
    #[case::middle_with_value("status --remote=mac --json")]
    #[case::last("status --json --remote")]
    #[case::last_with_value("status --json --remote=mac")]
    #[case::after_double_dash("new-tab -- echo --remote")]
    #[case::as_a_value("rename --name --remote")]
    #[case::with_an_empty_value("status --remote=")]
    fn refuses_remote_anywhere(#[case] line: &str) {
        assert_eq!(refusal_of(&argv(line)), REMOTE);
    }

    #[rstest]
    #[case::first("--remote-command cc status")]
    #[case::first_with_value("--remote-command=cc status")]
    #[case::middle("status --remote-command cc --json")]
    #[case::middle_with_value("status --remote-command=cc --json")]
    #[case::last("status --json --remote-command")]
    #[case::last_with_value("status --json --remote-command=cc")]
    #[case::after_double_dash("new-tab -- echo --remote-command")]
    #[case::as_a_value("rename --name --remote-command")]
    fn refuses_remote_command_anywhere(#[case] line: &str) {
        assert_eq!(refusal_of(&argv(line)), REMOTE_COMMAND);
    }

    #[test]
    fn names_remote_before_a_bad_command() {
        assert_eq!(refusal_of(&argv("kill-server --remote x")), REMOTE);
    }

    #[test]
    fn names_remote_before_an_empty_command() {
        assert_eq!(refusal_of(&argv("--remote")), REMOTE);
    }

    #[rstest]
    #[case::inside_one_argument(vec!["say --remote".to_owned()], "`say --remote` is not allowed through the bridge")]
    #[case::longer_flag(argv("--remotes"), "`--remotes` is not allowed through the bridge")]
    #[case::longer_flag_with_value(argv("--remotes=x"), "`--remotes=x` is not allowed through the bridge")]
    #[case::shorter_flag(argv("--rem"), "`--rem` is not allowed through the bridge")]
    fn leaves_lookalikes_to_the_other_rules(#[case] argv: Vec<String>, #[case] message: &str) {
        assert_eq!(refusal_of(&argv), message);
    }
}

mod empty_argv {
    use super::*;

    #[test]
    fn refuses_to_open_the_window() {
        assert_eq!(refusal_of(&[]), "no command: the bridge does not open the window");
    }
}

mod bare_flags {
    use super::*;

    #[rstest]
    #[case::help("--help")]
    #[case::short_help("-h")]
    #[case::version("--version")]
    #[case::short_version("-V")]
    fn allows_each_one_alone(#[case] line: &str) {
        assert_eq!(allowed(&argv(line)), Ok(()));
    }

    #[rstest]
    #[case::help_before_a_command("--help status", "`--help` is only allowed alone through the bridge")]
    #[case::short_help_before_a_command("-h status", "`-h` is only allowed alone through the bridge")]
    #[case::version_with_more("--version x", "`--version` is only allowed alone through the bridge")]
    #[case::short_version_with_more("-V x", "`-V` is only allowed alone through the bridge")]
    fn refuses_them_with_more_arguments(#[case] line: &str, #[case] message: &str) {
        assert_eq!(refusal_of(&argv(line)), message);
    }

    #[rstest]
    #[case::json("--json", "`--json` is not allowed through the bridge")]
    #[case::machine_first("--machine x status", "`--machine` is not allowed through the bridge")]
    #[case::lone_dash("-", "`-` is not allowed through the bridge")]
    #[case::uppercase_help("--HELP", "`--HELP` is not allowed through the bridge")]
    fn refuses_any_other_first_flag(#[case] line: &str, #[case] message: &str) {
        assert_eq!(refusal_of(&argv(line)), message);
    }
}

mod commands {
    use super::*;

    #[rstest]
    #[case::status("status --json", "`status` is not allowed through the bridge")]
    #[case::kill_server("kill-server", "`kill-server` is not allowed through the bridge")]
    #[case::unknown("nope", "`nope` is not allowed through the bridge")]
    fn refuses_every_name_until_the_list_exists(#[case] line: &str, #[case] message: &str) {
        assert_eq!(refusal_of(&argv(line)), message);
    }
}
