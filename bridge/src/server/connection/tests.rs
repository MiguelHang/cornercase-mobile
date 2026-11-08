use rstest::rstest;

use super::*;

mod command {
    use super::*;

    #[rstest]
    #[case::a_command(&["status", "--json"], "status")]
    #[case::a_bare_flag(&["--version"], "--version")]
    #[case::a_plugin_action(&["plugin", "run", "x/secret"], "plugin run")]
    #[case::plugin_alone(&["plugin"], "plugin")]
    #[case::nothing(&[], "")]
    fn names_only_the_first_word_or_the_plugin_action(#[case] argv: &[&str], #[case] name: &str) {
        let argv: Vec<String> = argv.iter().map(|&arg| arg.to_owned()).collect();
        assert_eq!(command(&argv), name);
    }
}
