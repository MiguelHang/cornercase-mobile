use rstest::rstest;

use super::*;
use crate::protocol::LINE_TOO_LONG;

const NO_REQUESTS_COMMAND: &str = "error: unrecognized subcommand 'requests'\n\nUsage: cornercase [OPTIONS]";
const NO_FOLLOW_FLAG: &str = "error: unexpected argument '--follow' found\n\nUsage: cornercase input";
const NO_SERVER: &str = "warning: slow\nError: no cornercase server is running\n\n";
const ENDED: &str = "`cornercase input --follow` ended";

fn exit(code: i32, stderr: &str) -> Exit {
    Exit { code, stderr: stderr.to_owned() }
}

fn crashed(code: i32, why: &str) -> Error {
    Error::Failed { code, stderr: why.to_owned() }
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
    #[case::another_usage_error(exit(2, LINE_TOO_LONG))]
    #[case::another_code(exit(1, "error: unrecognized subcommand 'requests'"))]
    #[case::a_dropped_server(exit(1, "Error: the connection to cornercase dropped"))]
    #[case::a_clean_end(exit(0, ""))]
    fn is_not_any_other_end(#[case] exit: Exit) {
        assert!(!absent(&exit));
    }
}

mod stopped {
    use super::*;

    #[rstest]
    #[case::its_last_line(Ok(exit(1, NO_SERVER)), crashed(1, "Error: no cornercase server is running"))]
    #[case::nothing_said(Ok(exit(0, "")), crashed(0, ENDED))]
    #[case::only_blank_lines(Ok(exit(0, "  \n\n")), crashed(0, ENDED))]
    #[case::a_dropped_connection(Err(Error::Dropped), Error::Dropped)]
    fn says_the_last_thing_the_process_wrote(#[case] end: End, #[case] error: Error) {
        assert_eq!(stopped(&end, ENDED), error);
    }
}
