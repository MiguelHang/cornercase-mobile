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
    #[case::contract("contract")]
    #[case::status("status --follow --json")]
    #[case::events("events --json")]
    #[case::read("read --last-message")]
    #[case::send("send --enter")]
    #[case::keys("keys")]
    #[case::paste_file("paste-file")]
    #[case::view("view")]
    #[case::view_set("view-set --claim")]
    #[case::surface("surface --patches")]
    #[case::input("input --follow --json -")]
    #[case::requests("requests --follow -")]
    #[case::scroll("scroll")]
    #[case::redraw("redraw")]
    #[case::start("start --worktree")]
    #[case::new_workspace("new-workspace")]
    #[case::issue_list("issue list")]
    #[case::issue_read("issue read")]
    #[case::changes_list("changes list")]
    #[case::changes_viewed("changes viewed")]
    #[case::files_list("files list")]
    #[case::files_read("files read")]
    #[case::files_search("files search")]
    #[case::stats("stats")]
    #[case::machine_after_the_command("status --machine vps")]
    fn allows_the_commands_of_the_app(#[case] line: &str) {
        assert_eq!(allowed(&argv(line)), Ok(()));
    }

    #[test]
    fn allows_every_control_command() {
        for name in CONTROL {
            assert_eq!(allowed(&[(*name).to_owned()]), Ok(()), "{name}");
        }
    }

    #[test]
    fn lists_exactly_the_control_commands_of_the_contract() {
        let expected = "status open clone new-workspace new-tab split setup start send keys held read wait close rename focus go notify away mark search todo queue events flow contract surface view view-set input requests scroll selection resize redraw ratio link paste-file theme config move move-pane collapse expand group issue changes files usage ports pull-request context recap processes worktrees machines tour agents account profile";
        assert_eq!(CONTROL.join(" "), expected);
    }

    #[rstest]
    #[case::start_server("start-server")]
    #[case::kill_server("kill-server")]
    #[case::update("update")]
    #[case::restart("restart")]
    #[case::logs("logs")]
    #[case::remote("remote")]
    #[case::skill("skill")]
    #[case::server("server")]
    #[case::proxy("proxy")]
    #[case::help("help")]
    #[case::other_case("Status")]
    #[case::prefix("stat")]
    #[case::unknown("nope")]
    fn refuses_the_rest(#[case] name: &str) {
        assert_eq!(refusal_of(&argv(name)), format!("`{name}` is not allowed through the bridge"));
    }
}

mod plugin {
    use super::*;

    #[rstest]
    #[case::actions("plugin actions")]
    #[case::status("plugin status --json")]
    #[case::run("plugin run some-plugin some-action")]
    #[case::popup_close("plugin popup-close")]
    fn allows_the_actions_that_ask_the_server(#[case] line: &str) {
        assert_eq!(allowed(&argv(line)), Ok(()));
    }

    #[rstest]
    #[case::windows("plugin windows")]
    #[case::link("plugin link")]
    #[case::install("plugin install")]
    #[case::update("plugin update")]
    #[case::remove("plugin remove")]
    #[case::unlink("plugin unlink")]
    #[case::list("plugin list")]
    #[case::enable("plugin enable")]
    #[case::disable("plugin disable")]
    #[case::logs("plugin logs")]
    #[case::an_option_first("plugin --json")]
    #[case::other_case("plugin Run")]
    fn refuses_the_other_actions(#[case] line: &str) {
        assert_eq!(refusal_of(&argv(line)), format!("`{line}` is not allowed through the bridge"));
    }

    #[test]
    fn lists_exactly_the_actions_that_ask_the_server() {
        assert_eq!(PLUGIN_ACTIONS, ["actions", "status", "run", "popup-close"]);
        assert_eq!(OTHERS, ["stats"]);
        assert_eq!(BARE_FLAGS, ["--help", "-h", "--version", "-V"]);
    }

    #[test]
    fn refuses_plugin_alone() {
        assert_eq!(refusal_of(&argv("plugin")), "`plugin` is not allowed through the bridge");
    }

    #[test]
    fn refuses_an_action_with_its_arguments_by_the_action_alone() {
        assert_eq!(refusal_of(&argv("plugin install x")), "`plugin install` is not allowed through the bridge");
    }
}

mod refusal_text {
    use super::*;

    #[rstest]
    #[case::newline_in_the_name(vec!["a\nb".to_owned()])]
    #[case::carriage_return_in_the_plugin_action(vec!["plugin".to_owned(), "x\ry".to_owned()])]
    #[case::control_characters_in_a_flag(vec!["-\u{1b}[2J".to_owned()])]
    fn is_a_single_line_even_when_the_name_holds_control_characters(#[case] argv: Vec<String>) {
        let message = refusal_of(&argv);
        assert!(!message.contains(char::is_control), "{message:?}");
    }

    #[test]
    fn cuts_a_long_name_after_eighty_characters() {
        let name = format!("{}ü{}", "a".repeat(79), "b".repeat(200));
        let expected = format!("`{}ü…` is not allowed through the bridge", "a".repeat(79));
        assert_eq!(refusal_of(&[name]), expected);
    }

    #[test]
    fn keeps_a_name_of_eighty_characters_whole() {
        let name = "a".repeat(80);
        assert_eq!(refusal_of(std::slice::from_ref(&name)), format!("`{name}` is not allowed through the bridge"));
    }
}
