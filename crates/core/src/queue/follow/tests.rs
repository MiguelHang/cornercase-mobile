use rstest::rstest;

use super::*;

const NO_REQUESTS_COMMAND: &str = "error: unrecognized subcommand 'requests'\n\nUsage: cornercase [OPTIONS]";
const NO_FOLLOW_FLAG: &str = "error: unexpected argument '--follow' found\n\nUsage: cornercase input";

fn exit(code: i32, stderr: &str) -> Exit {
    Exit { code, stderr: stderr.to_owned() }
}

mod absent {
    use super::*;

    #[rstest]
    #[case::no_requests_command(exit(2, NO_REQUESTS_COMMAND))]
    #[case::no_follow(exit(2, NO_FOLLOW_FLAG))]
    fn is_a_cli_that_does_not_know_the_command(#[case] exit: Exit) {
        assert!(absent(&exit));
    }

    #[rstest]
    #[case::another_usage_error(exit(2, "the bridge does not allow `kill-server`"))]
    #[case::another_code(exit(1, "error: unrecognized subcommand 'requests'"))]
    #[case::a_dropped_server(exit(1, "Error: the connection to cornercase dropped"))]
    #[case::a_clean_end(exit(0, ""))]
    fn is_not_any_other_end(#[case] exit: Exit) {
        assert!(!absent(&exit));
    }
}
