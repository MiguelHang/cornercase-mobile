use rstest::rstest;

use super::*;

mod bridge_message {
    use super::*;

    #[rstest]
    #[case::stdout(BridgeMessage::Stdout("a".to_owned()), r#"{"stdout":"a"}"#)]
    #[case::stderr(BridgeMessage::Stderr("b".to_owned()), r#"{"stderr":"b"}"#)]
    #[case::exit(BridgeMessage::Exit(2), r#"{"exit":2}"#)]
    #[case::negative_exit(BridgeMessage::Exit(-1), r#"{"exit":-1}"#)]
    fn is_an_object_with_one_snake_case_key(#[case] message: BridgeMessage, #[case] json: &str) {
        assert_eq!(serde_json::to_string(&message).expect("serialize the message"), json);
    }

    #[rstest]
    #[case::stdout(BridgeMessage::Stdout("line\n".to_owned()))]
    #[case::stderr(BridgeMessage::Stderr("error: ü".to_owned()))]
    #[case::exit(BridgeMessage::Exit(2))]
    #[case::negative_exit(BridgeMessage::Exit(-1))]
    fn reads_back_what_it_writes(#[case] message: BridgeMessage) {
        let json = serde_json::to_string(&message).expect("serialize the message");
        let read: BridgeMessage = serde_json::from_str(&json).expect("parse the message");
        assert_eq!(read, message);
    }
}

mod refusal {
    use super::*;

    fn refusal() -> Refusal {
        Refusal { message: "`kill-server` is not allowed through the bridge".to_owned() }
    }

    #[test]
    fn exits_with_the_wrong_usage_code_of_cornercase() {
        assert_eq!(USAGE_EXIT, 2);
    }

    #[test]
    fn displays_its_stderr_line() {
        assert_eq!(refusal().to_string(), "`kill-server` is not allowed through the bridge");
    }

    #[test]
    fn turns_control_characters_into_spaces() {
        assert_eq!(Refusal::new("`a\nb\rc\td` is not allowed").message, "`a b c d` is not allowed");
    }
}

mod client_message {
    use super::*;

    #[rstest]
    #[case::argv(r#"{"argv":["status","--json"]}"#, ClientMessage::Argv(vec!["status".to_owned(), "--json".to_owned()]))]
    #[case::empty_argv(r#"{"argv":[]}"#, ClientMessage::Argv(Vec::new()))]
    #[case::stdin(r#"{"stdin":"hello\n"}"#, ClientMessage::Stdin("hello\n".to_owned()))]
    #[case::stdin_base64_of_bytes_that_are_not_utf8(r#"{"stdin_base64":"//4A"}"#, ClientMessage::StdinBase64(vec![0xff, 0xfe, 0x00]))]
    #[case::eof(r#"{"eof":true}"#, ClientMessage::Eof)]
    fn reads_a_message_with_one_known_key(#[case] text: &str, #[case] message: ClientMessage) {
        assert_eq!(parse_client(text), Ok(message));
    }

    #[rstest]
    #[case::invalid_json(r#"{"argv":"#)]
    #[case::not_an_object_array("[]")]
    #[case::not_an_object_string(r#""x""#)]
    #[case::not_an_object_number("1")]
    #[case::not_an_object_key_name(r#""eof""#)]
    #[case::empty_object("{}")]
    #[case::unknown_key(r#"{"args":["status"]}"#)]
    #[case::two_keys_stdin_and_eof(r#"{"stdin":"a","eof":true}"#)]
    #[case::two_keys_argv_and_stdin(r#"{"argv":[],"stdin":"a"}"#)]
    #[case::argv_is_a_string(r#"{"argv":"status"}"#)]
    #[case::argv_holds_a_number(r#"{"argv":[1]}"#)]
    #[case::stdin_is_not_a_string(r#"{"stdin":1}"#)]
    #[case::eof_false(r#"{"eof":false}"#)]
    #[case::eof_not_bool(r#"{"eof":"true"}"#)]
    #[case::invalid_base64(r#"{"stdin_base64":"not base64!"}"#)]
    #[case::url_safe_base64(r#"{"stdin_base64":"-_4A"}"#)]
    fn refuses_anything_else_as_a_bad_message(#[case] text: &str) {
        let refusal = parse_client(text).expect_err("refuse the message");
        assert!(refusal.message.starts_with("bad message: "), "{}", refusal.message);
    }

    #[rstest]
    #[case::invalid_json(r#"{"argv":["~/x/secret"] secret"#, SYNTAX)]
    #[case::trailing_text(r#"{"eof":true} secret"#, SYNTAX)]
    #[case::cut_short(r#"{"argv":["~/x/secret""#, "bad message: the JSON ends too early")]
    #[case::wrong_type(r#"{"argv":"~/x/secret"}"#, DATA)]
    #[case::unknown_key(r#"{"secret":["status"]}"#, DATA)]
    #[case::two_keys(r#"{"stdin":"secret","eof":true}"#, SYNTAX)]
    #[case::invalid_base64(r#"{"stdin_base64":"secret!"}"#, "bad message: `stdin_base64` is not standard base64")]
    fn names_the_reason_without_echoing_the_message(#[case] text: &str, #[case] reason: &str) {
        let refusal = parse_client(text).expect_err("refuse the message");
        assert_eq!(refusal.message, reason);
    }

    const SYNTAX: &str = "bad message: not one JSON object with one key";
    const DATA: &str = "bad message: not one of `argv`, `stdin`, `stdin_base64` and `eof` with a value of its type";

    #[test]
    fn refuses_with_a_single_line_even_when_the_key_holds_a_newline() {
        let refusal = parse_client(r#"{"a\nb":1}"#).expect_err("refuse the message");
        assert!(!refusal.message.contains('\n'), "{}", refusal.message);
    }

    #[rstest]
    #[case::argv(ClientMessage::Argv(vec!["send".to_owned(), "--enter".to_owned()]))]
    #[case::stdin(ClientMessage::Stdin("ü\n".to_owned()))]
    #[case::stdin_base64(ClientMessage::StdinBase64(vec![0x00, 0xff, 0x10]))]
    #[case::eof(ClientMessage::Eof)]
    fn reads_back_what_it_writes(#[case] message: ClientMessage) {
        let json = serde_json::to_string(&message).expect("serialize the message");
        assert_eq!(parse_client(&json), Ok(message));
    }

    #[rstest]
    #[case::stdin_base64(ClientMessage::StdinBase64(vec![0xff, 0xfe, 0x00]), r#"{"stdin_base64":"//4A"}"#)]
    #[case::eof(ClientMessage::Eof, r#"{"eof":true}"#)]
    fn writes_bytes_in_standard_base64_and_eof_as_true(#[case] message: ClientMessage, #[case] json: &str) {
        assert_eq!(serde_json::to_string(&message).expect("serialize the message"), json);
    }
}

mod message_order {
    use super::*;

    #[test]
    fn takes_the_argv_from_the_first_message() {
        assert_eq!(
            first_message(r#"{"argv":["status","--json"]}"#),
            Ok(vec!["status".to_owned(), "--json".to_owned()])
        );
    }

    #[test]
    fn leaves_an_empty_argv_to_the_allowlist() {
        assert_eq!(first_message(r#"{"argv":[]}"#), Ok(Vec::new()));
    }

    #[rstest]
    #[case::stdin_first(r#"{"stdin":"a"}"#)]
    #[case::stdin_base64_first(r#"{"stdin_base64":"//4A"}"#)]
    #[case::eof_first(r#"{"eof":true}"#)]
    fn refuses_a_first_message_that_is_not_argv(#[case] text: &str) {
        let refusal = first_message(text).expect_err("refuse the first message");
        assert_eq!(refusal.message, "the first message must be `argv`");
    }

    #[test]
    fn refuses_invalid_json_first_as_a_bad_message() {
        let refusal = first_message(r#"{"argv":"#).expect_err("refuse the first message");
        assert!(refusal.message.starts_with("bad message: "), "{}", refusal.message);
    }

    #[rstest]
    #[case::text(r#"{"stdin":"hello\n"}"#, Input::Text("hello\n".to_owned()))]
    #[case::bytes(r#"{"stdin_base64":"//4A"}"#, Input::Bytes(vec![0xff, 0xfe, 0x00]))]
    #[case::eof(r#"{"eof":true}"#, Input::Eof)]
    fn reads_each_input_after_the_argv(#[case] text: &str, #[case] input: Input) {
        assert_eq!(next_message(text), Ok(input));
    }

    #[rstest]
    #[case::second_argv(r#"{"argv":["status"]}"#)]
    #[case::second_empty_argv(r#"{"argv":[]}"#)]
    fn refuses_a_second_argv(#[case] text: &str) {
        let refusal = next_message(text).expect_err("refuse the second argv");
        assert_eq!(refusal.message, "`argv` was already sent");
    }

    #[test]
    fn refuses_invalid_json_after_the_argv_as_a_bad_message() {
        let refusal = next_message("{}").expect_err("refuse the message");
        assert!(refusal.message.starts_with("bad message: "), "{}", refusal.message);
    }
}
