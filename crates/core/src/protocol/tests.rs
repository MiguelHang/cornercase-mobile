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
}
