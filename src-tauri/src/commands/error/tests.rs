use cornercase_core::machines::route::Route;
use rstest::rstest;
use serde_json::json;

use super::*;

const TOKEN: &str = "c3R1ZGlvLXNlY3JldA";

fn json_of(error: &CommandError) -> serde_json::Value {
    serde_json::to_value(error).expect("an error serializes")
}

mod from_a_core_error {
    use super::*;

    #[rstest]
    #[case::unauthorized(Error::Unauthorized, "unauthorized")]
    #[case::dropped(Error::Dropped, "dropped")]
    #[case::output_closed(Error::OutputClosed, "output_closed")]
    #[case::rejected(Error::Rejected { status: 503 }, "rejected")]
    #[case::timeout(Error::Timeout, "timeout")]
    #[case::connect(Error::Connect("refused".into()), "connect")]
    #[case::protocol(Error::Protocol("a binary frame".into()), "protocol")]
    #[case::failed(Error::Failed { code: 2, stderr: "no such pane".into() }, "failed")]
    #[case::stale_boot(Error::StaleBoot("boot 7".into()), "stale_boot")]
    #[case::stream_ended(Error::StreamEnded, "stream_ended")]
    #[case::outdated(Error::Outdated, "outdated")]
    #[case::contract_too_new(Error::ContractTooNew { server: 9, app: 4 }, "contract_too_new")]
    #[case::unanswered(Error::Unanswered, "unanswered")]
    #[case::settle_timeout(Error::SettleTimeout, "settle_timeout")]
    #[case::input_rejected(Error::InputRejected("closed".into()), "input_rejected")]
    #[case::json(Error::Json("expected a value".into()), "json")]
    #[case::store(Error::Store("read-only".into()), "store")]
    #[case::secrets(Error::Secrets("locked".into()), "secrets")]
    #[case::no_token(Error::NoToken, "no_token")]
    #[case::unknown_machine(Error::UnknownMachine, "unknown_machine")]
    #[case::unknown_route(Error::UnknownRoute, "unknown_route")]
    #[case::identity(Error::Identity, "identity")]
    #[case::random(Error::Random, "random")]
    fn carries_its_code_and_its_message(#[case] error: Error, #[case] code: &str) {
        let message = error.to_string();
        assert_eq!(json_of(&CommandError::from(error)), json!({ "code": code, "message": message }));
    }
}

mod from_a_route_error {
    use super::*;

    #[rstest]
    #[case::not_a_url("studio lan c3R1ZGlvLXNlY3JldA", "route_not_a_url")]
    #[case::scheme("http://studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA", "route_scheme")]
    #[case::user_info("ws://me@studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA", "route_user_info")]
    #[case::no_host("ws://:9000/?token=c3R1ZGlvLXNlY3JldA", "route_no_host")]
    #[case::port("ws://studio.lan:99999/?token=c3R1ZGlvLXNlY3JldA", "route_port")]
    #[case::path("ws://studio.lan:9000/bridge?token=c3R1ZGlvLXNlY3JldA", "route_path")]
    #[case::no_token("ws://studio.lan:9000/?key=c3R1ZGlvLXNlY3JldA", "route_no_token")]
    #[case::empty_token("ws://studio.lan:9000/?token=", "route_empty_token")]
    #[case::token_twice("ws://studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA&token=c3R1ZGlvLXNlY3JldA", "route_token_twice")]
    #[case::token_characters("ws://studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA%3D", "route_token_characters")]
    fn carries_its_code_and_a_message_without_the_url(#[case] url: &str, #[case] code: &str) {
        let error = Route::parse(url).expect_err("a route that does not parse");
        let json = json_of(&CommandError::from(error));
        assert_eq!(json, json!({ "code": code, "message": error.to_string() }));
        let message = json["message"].as_str().expect("a message");
        assert!(!message.contains("studio"), "{message}");
        assert!(!message.contains(TOKEN), "{message}");
    }
}

mod of_the_app {
    use super::*;

    #[rstest]
    #[case::no_data_dir(
        CommandError::no_data_dir("no home folder"),
        "no_data_dir",
        "the app has no folder for its data: no home folder"
    )]
    #[case::machines_not_loaded(
        CommandError::machines_not_loaded(&Error::Store("locked".into())),
        "machines_not_loaded",
        "the saved machines are not loaded: cannot read or write the saved machines: locked"
    )]
    #[case::no_identity(
        CommandError::no_identity(&Error::Secrets("locked".into())),
        "no_identity",
        "the app has no identity: the secret store failed: locked"
    )]
    #[case::not_connected(CommandError::not_connected(), "not_connected", "no machine is connected")]
    #[case::not_sent(CommandError::not_sent(), "not_sent", "nothing was sent")]
    #[case::maybe_sent(CommandError::maybe_sent(), "unanswered", "it may have been sent, but no answer came back")]
    fn carries_its_code_and_its_message(#[case] error: CommandError, #[case] code: &str, #[case] message: &str) {
        assert_eq!(json_of(&error), json!({ "code": code, "message": message }));
    }
}

mod with_the_reason_of_cornercase {
    use super::*;

    fn failed(stderr: &str) -> Error {
        Error::Failed { code: 1, stderr: stderr.to_owned() }
    }

    #[rstest]
    #[case::without_its_prefix("Error: pane 543 is waiting\n", "pane 543 is waiting")]
    #[case::as_it_is("pane 543 is waiting", "pane 543 is waiting")]
    #[case::trimmed("  pane 543 is waiting  ", "pane 543 is waiting")]
    fn says_what_cornercase_said(#[case] stderr: &str, #[case] reason: &str) {
        assert_eq!(json_of(&CommandError::reason(&failed(stderr))), json!({ "code": "failed", "message": reason }));
    }

    #[rstest]
    #[case::empty("")]
    #[case::blank(" \n")]
    #[case::only_the_prefix("Error:")]
    fn keeps_the_whole_failure_when_cornercase_said_nothing(#[case] stderr: &str) {
        let error = failed(stderr);
        assert_eq!(json_of(&CommandError::reason(&error)), json_of(&CommandError::from(&error)));
    }

    #[test]
    fn leaves_any_other_error_as_it_is() {
        assert_eq!(CommandError::reason(&Error::Unanswered), CommandError::from(Error::Unanswered));
    }
}
