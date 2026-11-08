use std::time::Duration;

use rstest::rstest;
use tokio::io::AsyncRead;
use tokio::time::timeout;

use super::*;

const PATIENCE: Duration = Duration::from_secs(5);

async fn read<R>(stream: R, message: fn(String) -> BridgeMessage) -> Vec<BridgeMessage>
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
    let ((), messages) = tokio::join!(forward(stream, message, sender), collect);
    messages
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

        timeout(PATIENCE, forward(endless, BridgeMessage::Stdout, sender)).await.expect("forward returns");
    }
}
