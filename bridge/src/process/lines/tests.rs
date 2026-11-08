use std::time::Duration;

use rstest::rstest;
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio::time::{Instant, timeout};

use cornercase_core::protocol::LONGEST_LINE;

use super::*;

const PATIENCE: Duration = Duration::from_secs(5);

async fn read<R>(stream: R, message: fn(String) -> BridgeMessage) -> Vec<BridgeMessage>
where
    R: AsyncRead + Unpin,
{
    read_by(stream, message, None).await
}

async fn read_by<R>(stream: R, message: fn(String) -> BridgeMessage, deadline: Option<Instant>) -> Vec<BridgeMessage>
where
    R: AsyncRead + Unpin,
{
    read_all(stream, message, deadline, LONGEST_LINE).await.0
}

async fn read_capped<R>(stream: R, longest: usize) -> (Vec<BridgeMessage>, End)
where
    R: AsyncRead + Unpin,
{
    read_all(stream, BridgeMessage::Stdout, None, longest).await
}

async fn read_all<R>(
    stream: R,
    message: fn(String) -> BridgeMessage,
    deadline: Option<Instant>,
    longest: usize,
) -> (Vec<BridgeMessage>, End)
where
    R: AsyncRead + Unpin,
{
    let (sender, mut receiver) = mpsc::channel(1);
    let collect = async {
        let mut messages = Vec::new();
        while let Some(message) = receiver.recv().await {
            messages.push(message);
        }
        messages
    };
    let (end, messages) = tokio::join!(forward(stream, message, sender, watch::channel(deadline).1, longest), collect);
    (messages, end)
}

fn stdout(lines: &[&str]) -> Vec<BridgeMessage> {
    lines.iter().map(|line| BridgeMessage::Stdout((*line).to_owned())).collect()
}

mod forward {
    use super::*;

    #[rstest]
    #[case::two_lines(b"a\nb\n", &["a", "b"])]
    #[case::an_empty_line_in_the_middle(b"a\n\nb\n", &["a", "", "b"])]
    #[case::a_carriage_return_is_kept(b"a\r\n", &["a\r"])]
    #[case::a_last_line_without_a_newline(b"a\nb", &["a", "b"])]
    #[case::a_lone_newline(b"\n", &[""])]
    #[case::nothing(b"", &[])]
    #[case::invalid_utf8_is_replaced(b"a\xffb\nc\n", &["a\u{FFFD}b", "c"])]
    #[tokio::test]
    async fn sends_each_line_without_its_newline(#[case] bytes: &[u8], #[case] lines: &[&str]) {
        assert_eq!(read(bytes, BridgeMessage::Stdout).await, stdout(lines));
    }

    #[tokio::test]
    async fn wraps_each_line_with_the_stream_s_message() {
        let received = read(&b"oops\n"[..], BridgeMessage::Stderr).await;

        assert_eq!(received, [BridgeMessage::Stderr("oops".to_owned())]);
    }

    #[tokio::test]
    async fn sends_a_line_longer_than_the_buffer_in_one_message() {
        let long = "x".repeat(BUFFER + 1);
        let bytes = format!("{long}\nend\n");

        assert_eq!(read(bytes.as_bytes(), BridgeMessage::Stdout).await, stdout(&[&long, "end"]));
    }

    #[tokio::test]
    async fn stops_once_nobody_receives() {
        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);

        let endless = tokio::io::repeat(b'\n');

        let unset = watch::channel(None).1;

        let end = timeout(PATIENCE, forward(endless, BridgeMessage::Stdout, sender, unset, LONGEST_LINE)).await;

        assert_eq!(end.ok(), Some(End::Closed));
    }

    #[tokio::test]
    async fn reads_what_is_ready_past_the_deadline() {
        let lines = ["a"; 100].join("\n");

        assert_eq!(read_by(lines.as_bytes(), BridgeMessage::Stdout, Some(Instant::now())).await, stdout(&["a"; 100]));
    }

    #[tokio::test]
    async fn stops_waiting_for_a_stream_still_open_past_the_deadline() {
        let (mut writer, stream) = tokio::io::duplex(64);
        writer.write_all(b"a\n").await.expect("write a line");

        let lines = timeout(PATIENCE, read_by(stream, BridgeMessage::Stdout, Some(Instant::now()))).await;

        assert_eq!(lines.ok(), Some(stdout(&["a"])));
    }

    #[rstest]
    #[case::with_its_newline("\n")]
    #[case::at_the_end_of_the_stream("")]
    #[tokio::test]
    async fn sends_a_line_as_long_as_the_limit_whole(#[case] tail: &str) {
        let line = "x".repeat(BUFFER + 1);
        let bytes = format!("a\n{line}{tail}");

        assert_eq!(read_capped(bytes.as_bytes(), BUFFER + 1).await, (stdout(&["a", &line]), End::Closed));
    }

    #[rstest]
    #[case::with_its_newline("\nb\n")]
    #[case::at_the_end_of_the_stream("")]
    #[tokio::test]
    async fn stops_at_a_line_one_byte_past_the_limit(#[case] tail: &str) {
        let bytes = format!("a\n{}{tail}", "x".repeat(BUFFER + 2));

        assert_eq!(read_capped(bytes.as_bytes(), BUFFER + 1).await, (stdout(&["a"]), End::TooLong));
    }

    #[tokio::test]
    async fn stops_at_a_line_that_never_ends() {
        let endless = tokio::io::repeat(b'x');

        let read = timeout(PATIENCE, read_capped(endless, BUFFER + 1)).await;

        assert_eq!(read.ok(), Some((Vec::new(), End::TooLong)));
    }
}
