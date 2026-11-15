use std::io;

use rstest::rstest;
use tokio_tungstenite::tungstenite::error::CapacityError;

use super::*;
use crate::transport::fake::cli::request;

fn bytes(length: usize) -> Vec<u8> {
    (0..=u8::MAX).cycle().take(length).collect()
}

fn texts(input: Stdin) -> Vec<String> {
    messages(input).iter().map(|message| serde_json::to_string(message).expect("a message")).collect()
}

mod messages {
    use super::*;

    #[rstest]
    #[case::nothing(0, 0)]
    #[case::one_byte(1, 1)]
    #[case::a_full_piece(PIECE, 1)]
    #[case::one_byte_more(PIECE + 1, 2)]
    #[case::several_pieces(3 * PIECE + 2, 4)]
    fn cut_the_bytes_into_messages_of_one_mib_at_most(#[case] length: usize, #[case] count: usize) {
        let texts = texts(Stdin::Bytes(bytes(length)));
        assert_eq!(texts.len(), count);
        assert!(texts.iter().all(|text| text.len() <= LARGEST_MESSAGE), "a message passed 1 MiB");
        let mut joined = Vec::new();
        for input in request(&texts).stdin {
            match input {
                Stdin::Bytes(piece) => joined.extend(piece),
                Stdin::Text(text) => panic!("not bytes: {text:?}"),
            }
        }
        assert_eq!(joined, bytes(length));
    }

    #[test]
    fn fill_the_message_with_a_full_piece() {
        let texts = texts(Stdin::Bytes(bytes(PIECE)));
        assert!(LARGEST_MESSAGE - texts[0].len() < 4, "another group of base64 fits in {}", texts[0].len());
    }

    #[test]
    fn send_a_text_as_it_is() {
        assert_eq!(messages(Stdin::Text("one two".to_owned())), vec![ClientMessage::Stdin("one two".to_owned())]);
    }
}

mod heard {
    use super::*;

    const NOT_A_SERVER_MESSAGE: &str = "not one `stdout`, `stderr` or `exit` with a value of its type";

    fn unexpected(what: &str) -> Result<Option<ServerMessage>, Error> {
        Err(Error::Protocol(what.to_owned()))
    }

    #[rstest]
    #[case::stdout(r#"{"stdout":"one"}"#, Ok(Some(ServerMessage::Stdout("one".to_owned()))))]
    #[case::stderr(r#"{"stderr":"two"}"#, Ok(Some(ServerMessage::Stderr("two".to_owned()))))]
    #[case::exit(r#"{"exit":3}"#, Ok(Some(ServerMessage::Exit(3))))]
    #[case::not_json("secret", unexpected(NOT_A_SERVER_MESSAGE))]
    #[case::unknown_key(r#"{"secret":"x"}"#, unexpected(NOT_A_SERVER_MESSAGE))]
    #[case::wrong_type(r#"{"exit":"secret"}"#, unexpected(NOT_A_SERVER_MESSAGE))]
    #[case::two_keys(r#"{"exit":0,"stdout":"secret"}"#, unexpected(NOT_A_SERVER_MESSAGE))]
    fn translates_each_text(#[case] text: &str, #[case] expected: Result<Option<ServerMessage>, Error>) {
        assert_eq!(heard(Some(Ok(Message::text(text)))), expected);
    }

    #[rstest]
    #[case::ping(Message::Ping(Vec::new().into()), Ok(None))]
    #[case::pong(Message::Pong(Vec::new().into()), Ok(None))]
    #[case::close(Message::Close(None), Err(Error::Dropped))]
    #[case::binary(Message::binary(br#"{"exit":0}"#.to_vec()), unexpected("a binary frame"))]
    fn translates_each_frame(#[case] message: Message, #[case] expected: Result<Option<ServerMessage>, Error>) {
        assert_eq!(heard(Some(Ok(message))), expected);
    }

    #[test]
    fn drops_when_the_stream_ends() {
        assert_eq!(heard(None), Err(Error::Dropped));
    }

    #[rstest]
    #[case::reset(WebSocketError::Io(io::ErrorKind::ConnectionReset.into()), Err(Error::Dropped))]
    #[case::no_closing_handshake(
        WebSocketError::Protocol(ProtocolError::ResetWithoutClosingHandshake),
        Err(Error::Dropped)
    )]
    #[case::already_closed(WebSocketError::AlreadyClosed, Err(Error::Dropped))]
    #[case::too_long(
        WebSocketError::Capacity(CapacityError::MessageTooLong { size: 2, max_size: 1 }),
        unexpected("Space limit exceeded: Message too long: 2 > 1")
    )]
    #[case::broken_frame(
        WebSocketError::Protocol(ProtocolError::MaskedFrameFromServer),
        unexpected("WebSocket protocol error: Received a masked frame from server")
    )]
    #[case::bad_text(WebSocketError::Utf8("bad".to_owned()), unexpected("UTF-8 encoding error: bad"))]
    fn translates_each_failure(#[case] error: WebSocketError, #[case] expected: Result<Option<ServerMessage>, Error>) {
        assert_eq!(heard(Some(Err(error))), expected);
    }
}
