use rstest::rstest;

use super::*;

mod display {
    use super::*;

    #[rstest]
    #[case::unauthorized(Error::Unauthorized, "the bridge refused the token")]
    #[case::dropped(Error::Dropped, "the connection to the bridge dropped before the command finished")]
    #[case::output_closed(Error::OutputClosed, "nothing listens to the command's output any more")]
    #[case::rejected(Error::Rejected { status: 404 }, "the bridge answered with HTTP 404")]
    #[case::timeout(Error::Timeout, "the bridge did not answer in time")]
    #[case::connect(Error::Connect("connection refused".to_owned()), "cannot reach the bridge: connection refused")]
    #[case::protocol(
        Error::Protocol("a binary frame".to_owned()),
        "the bridge sent an unexpected message: a binary frame"
    )]
    #[case::failed(
        Error::Failed { code: 3, stderr: "no such project".to_owned() },
        "cornercase exited with code 3: no such project"
    )]
    #[case::json(
        Error::Json("expected value at line 1 column 1".to_owned()),
        "cornercase's output is not the expected JSON: expected value at line 1 column 1"
    )]
    #[case::store(
        Error::Store("permission denied".to_owned()),
        "cannot read or write the saved machines: permission denied"
    )]
    #[case::secrets(Error::Secrets("item not found".to_owned()), "the secret store failed: item not found")]
    #[case::no_token(Error::NoToken, "the route has no saved token; scan the bridge's code again")]
    fn says_what_failed(#[case] error: Error, #[case] message: &str) {
        assert_eq!(error.to_string(), message);
    }
}
