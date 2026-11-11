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
    #[case::stale_boot(
        Error::StaleBoot("error: the server restarted".to_owned()),
        "the cornercase server restarted since the command was prepared: error: the server restarted"
    )]
    #[case::stream_ended(Error::StreamEnded, "cornercase ended a stream the app was following")]
    #[case::outdated(
        Error::Outdated,
        "cornercase on the Mac is too old for this app; update cornercase on the Mac and restart its server"
    )]
    #[case::contract_too_new(
        Error::ContractTooNew { server: 2, app: 1 },
        "cornercase speaks contract 2 but this app knows up to 1; update the app"
    )]
    #[case::unanswered(Error::Unanswered, "the command may have run but its answer was lost")]
    #[case::settle_timeout(
        Error::SettleTimeout,
        "the terminal input got no answer in time, so the next command did not wait for it"
    )]
    #[case::input_rejected(
        Error::InputRejected("no pane 14".to_owned()),
        "cornercase input: no pane 14"
    )]
    #[case::unknown_key(Error::UnknownKey("--force".to_owned()), "the app sends no key named \"--force\"")]
    #[case::no_keys(Error::NoKeys, "no key to send")]
    #[case::image_empty(Error::ImageEmpty, "the image is empty: there is nothing to paste")]
    #[case::image_too_large(Error::ImageTooLarge, "the image is over 32 MiB, the most cornercase pastes")]
    #[case::picker(
        Error::Picker("only the iOS app has a photo picker".to_owned()),
        "cannot pick a photo: only the iOS app has a photo picker"
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
    #[case::unknown_machine(Error::UnknownMachine, "no saved machine has that id")]
    #[case::unknown_route(Error::UnknownRoute, "the machine has no route at that address")]
    #[case::last_route(Error::LastRoute, "a machine keeps at least one route; remove the machine instead")]
    #[case::identity(Error::Identity, "the saved identity is not a 32-byte Ed25519 seed")]
    #[case::random(Error::Random, "the system gave no random bytes for the identity")]
    fn says_what_failed(#[case] error: Error, #[case] message: &str) {
        assert_eq!(error.to_string(), message);
    }
}
