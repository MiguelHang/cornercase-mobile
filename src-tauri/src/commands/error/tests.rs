use cornercase_core::machines::code::MachineCode;
use cornercase_core::machines::route::Address;
use rstest::rstest;
use serde_json::json;

use super::*;

const TOKEN: &str = "c3R1ZGlvLXNlY3JldA";
const PAIRING_CODE: &str = "0123456789abcdef0123456789abcdef";
const SECRET: &str = "9c1e2a7b.5d0f3c8e9a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6071";

fn json_of(error: &CommandError) -> serde_json::Value {
    serde_json::to_value(error).expect("an error serializes")
}

mod from_a_core_error {
    use super::*;

    #[rstest]
    #[case::not_paired(Error::NotPaired, "not_paired")]
    #[case::dropped(Error::Dropped, "dropped")]
    #[case::output_closed(Error::OutputClosed, "output_closed")]
    #[case::rejected(Error::Rejected { status: 503 }, "rejected")]
    #[case::timeout(Error::Timeout, "timeout")]
    #[case::connect(Error::Connect("refused".into()), "connect")]
    #[case::protocol(Error::Protocol("a binary frame".into()), "protocol")]
    #[case::malformed_secret(Error::MalformedSecret, "protocol")]
    #[case::pairing_refused(Error::PairingRefused, "pairing_refused")]
    #[case::pairing_failed(Error::PairingFailed, "pairing_failed")]
    #[case::failed(Error::Failed { code: 2, stderr: "no such pane".into() }, "failed")]
    #[case::stale_boot(Error::StaleBoot("boot 7".into()), "stale_boot")]
    #[case::stream_ended(Error::StreamEnded, "stream_ended")]
    #[case::outdated(Error::Outdated, "outdated")]
    #[case::contract_too_new(Error::ContractTooNew { server: 9, app: 4 }, "contract_too_new")]
    #[case::unanswered(Error::Unanswered, "unanswered")]
    #[case::settle_timeout(Error::SettleTimeout, "settle_timeout")]
    #[case::input_rejected(Error::InputRejected("closed".into()), "input_rejected")]
    #[case::unknown_key(Error::UnknownKey("--force".into()), "unknown_key")]
    #[case::no_keys(Error::NoKeys, "no_keys")]
    #[case::image_empty(Error::ImageEmpty, "image_empty")]
    #[case::image_too_large(Error::ImageTooLarge, "image_too_large")]
    #[case::picker(Error::Picker("cancelled by the system".into()), "picker")]
    #[case::json(Error::Json("expected a value".into()), "json")]
    #[case::store(Error::Store("read-only".into()), "store")]
    #[case::secrets(Error::Secrets("locked".into()), "secrets")]
    #[case::no_token(Error::NoToken, "no_token")]
    #[case::unknown_machine(Error::UnknownMachine, "unknown_machine")]
    #[case::unknown_route(Error::UnknownRoute, "unknown_route")]
    #[case::last_route(Error::LastRoute, "last_route")]
    #[case::name_empty(Error::NameEmpty, "name_empty")]
    #[case::name_too_long(Error::NameTooLong, "name_too_long")]
    #[case::name_taken(Error::NameTaken, "name_taken")]
    #[case::identity(Error::Identity, "identity")]
    #[case::random(Error::Random, "random")]
    fn carries_its_code_and_its_message(#[case] error: Error, #[case] code: &str) {
        let message = error.to_string();
        assert_eq!(json_of(&CommandError::from(error)), json!({ "code": code, "message": message }));
    }
}

mod of_pairing {
    use super::*;

    #[rstest]
    #[case::not_paired(Error::NotPaired)]
    #[case::malformed_secret(Error::MalformedSecret)]
    #[case::pairing_refused(Error::PairingRefused)]
    #[case::pairing_failed(Error::PairingFailed)]
    fn never_says_the_code_or_the_secret(#[case] error: Error) {
        let json = json_of(&CommandError::from(error));
        let message = json["message"].as_str().expect("a message");
        assert!(!message.contains(PAIRING_CODE), "{message}");
        assert!(!message.contains(SECRET), "{message}");
    }

    #[rstest]
    #[case::refused(PairError::Answered(Error::PairingRefused), "pairing_refused")]
    #[case::failed(PairError::Answered(Error::PairingFailed), "pairing_failed")]
    #[case::malformed(PairError::Answered(Error::MalformedSecret), "protocol")]
    fn carries_the_code_of_what_the_mac_answered(#[case] error: PairError, #[case] code: &str) {
        let message = error.to_string();
        assert_eq!(json_of(&CommandError::from(error)), json!({ "code": code, "message": message }));
    }

    #[test]
    fn says_where_to_look_when_no_address_answered_without_naming_them() {
        let unanswered = vec![(Address { host: "mac.lan".to_owned(), port: 47682 }, Error::Timeout)];
        let json = json_of(&CommandError::from(PairError::NoAnswer(unanswered)));
        let message = json["message"].as_str().expect("a message");
        assert_eq!(json["code"], "connect");
        assert!(message.contains("phone app"), "{message}");
        assert!(!message.contains("mac.lan"), "{message}");
    }
}

mod from_a_route_error {
    use super::*;

    #[rstest]
    #[case::not_a_url("studio lan c0de", "route_not_a_url")]
    #[case::scheme("http://studio.lan:9000/?code=c0de", "route_scheme")]
    #[case::user_info("ws://me@studio.lan:9000/?code=c0de", "route_user_info")]
    #[case::no_host("ws://:9000/?code=c0de", "route_no_host")]
    #[case::port("ws://studio.lan:99999/?code=c0de", "route_port")]
    #[case::path("ws://studio.lan:9000/cli?code=c0de", "route_path")]
    #[case::also_empty("ws://studio.lan:9000/?code=c0de&also=", "route_also_empty")]
    #[case::also_invalid("ws://studio.lan:9000/?code=c0de&also=[::1]", "route_also_invalid")]
    #[case::name_encoding("ws://studio.lan:9000/?code=c0de&name=%FF", "route_name_encoding")]
    #[case::name_twice("ws://studio.lan:9000/?code=c0de&name=a&name=b", "route_name_twice")]
    #[case::no_code("ws://studio.lan:9000/?also=10.0.0.2", "route_no_code")]
    #[case::empty_code("ws://studio.lan:9000/?code=", "route_empty_code")]
    #[case::code_twice("ws://studio.lan:9000/?code=c0de&code=c0de", "route_code_twice")]
    #[case::code_characters("ws://studio.lan:9000/?code=C0DE", "route_code_characters")]
    fn carries_the_code_of_a_machine_code_error(#[case] text: &str, #[case] code: &str) {
        assert_carries(MachineCode::parse(text).expect_err("a code that does not parse"), code);
    }

    fn assert_carries(error: RouteError, code: &str) {
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
    #[case::no_digest(
        CommandError::no_digest(),
        "no_digest",
        "marking a file viewed needs the digest of the version shown"
    )]
    #[case::too_large_here(CommandError::too_large_here(), "too_large_here", "Too large to show on the phone")]
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
