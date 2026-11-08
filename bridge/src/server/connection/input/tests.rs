use rstest::rstest;

use super::*;

mod input {
    use super::*;

    #[rstest]
    #[case::stdin(Message::text(r#"{"stdin":"hi"}"#), Some(Input::Text("hi".to_owned())))]
    #[case::stdin_base64(Message::text(r#"{"stdin_base64":"aGk="}"#), Some(Input::Bytes(b"hi".to_vec())))]
    #[case::eof(Message::text(r#"{"eof":true}"#), Some(Input::Eof))]
    #[case::a_ping(Message::Ping(Vec::new().into()), None)]
    #[case::a_pong(Message::Pong(Vec::new().into()), None)]
    #[case::a_close(Message::Close(None), None)]
    fn takes_input_and_skips_control_frames(#[case] message: Message, #[case] taken: Option<Input>) {
        assert_eq!(input(message), Ok(taken));
    }

    #[rstest]
    #[case::a_second_argv(Message::text(r#"{"argv":["status"]}"#), "`argv` was already sent")]
    #[case::a_binary_frame(Message::binary(vec![1]), "a message must be text, not binary")]
    #[case::bad_json(Message::text("{"), "bad message: the JSON ends too early")]
    fn refuses_a_bad_message(#[case] message: Message, #[case] reason: &str) {
        assert_eq!(input(message), Err(Refusal::new(reason)));
    }
}
