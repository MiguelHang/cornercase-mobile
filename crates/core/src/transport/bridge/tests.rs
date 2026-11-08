use std::io;
use std::time::Duration;

use rstest::rstest;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::error::{ProtocolError, UrlError};
use tokio_tungstenite::tungstenite::handshake::server::{Callback, ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http;

use super::*;

const URL: &str = "ws://bridge.example:7777/?token=secret";

fn refusal(status: u16) -> WebSocketError {
    let response = http::Response::builder().status(status).body(None).expect("a response");
    WebSocketError::Http(Box::new(response))
}

async fn listener() -> (String, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a listener");
    let url = format!("ws://{}/?token=secret", listener.local_addr().expect("its address"));
    (url, listener)
}

struct Answer(Option<u16>);

impl Callback for Answer {
    fn on_request(self, _: &Request, response: Response) -> Result<Response, ErrorResponse> {
        match self.0 {
            None => Ok(response),
            Some(status) => Err(http::Response::builder().status(status).body(None).expect("a refusal")),
        }
    }
}

async fn upgrade(listener: TcpListener, answer: Option<u16>) {
    let (stream, _) = listener.accept().await.expect("a connection");
    let _ = accept_hdr_async(stream, Answer(answer)).await;
}

async fn connect_to(answer: Option<u16>) -> Result<Socket, Error> {
    let (url, listener) = listener().await;
    tokio::spawn(upgrade(listener, answer));
    BridgeTransport::new(url, BridgeConfig::default()).connect().await
}

mod config {
    use super::*;

    #[test]
    fn waits_three_seconds_to_connect_by_default() {
        assert_eq!(BridgeConfig::default().connect_timeout, Duration::from_secs(3));
    }

    #[test]
    fn takes_frames_and_messages_of_seven_longest_lines() {
        let config = websocket();
        assert_eq!(config.max_frame_size, Some(7 * LONGEST_LINE));
        assert_eq!(config.max_message_size, Some(7 * LONGEST_LINE));
    }
}

mod connect {
    use super::*;

    #[tokio::test]
    async fn upgrades_to_a_websocket() {
        assert!(connect_to(None).await.is_ok(), "the upgrade failed");
    }

    #[rstest]
    #[case::unauthorized(401, Error::Unauthorized)]
    #[case::not_found(404, Error::Rejected { status: 404 })]
    #[case::unavailable(503, Error::Rejected { status: 503 })]
    #[tokio::test]
    async fn tells_a_refused_token_from_another_refusal(#[case] status: u16, #[case] expected: Error) {
        assert_eq!(connect_to(Some(status)).await.err(), Some(expected));
    }

    #[tokio::test]
    async fn gives_up_when_the_upgrade_does_not_come_in_time() {
        let (url, _listener) = listener().await;
        let config = BridgeConfig { connect_timeout: Duration::from_millis(100) };
        assert_eq!(BridgeTransport::new(url, config).connect().await.err(), Some(Error::Timeout));
    }
}

mod connect_error {
    use super::*;

    #[rstest]
    #[case::unauthorized(refusal(401), Error::Unauthorized)]
    #[case::not_found(refusal(404), Error::Rejected { status: 404 })]
    #[case::os_error(
        WebSocketError::Io(io::Error::from_raw_os_error(1)),
        Error::Connect(io::Error::from_raw_os_error(1).to_string())
    )]
    #[case::other_io(WebSocketError::Io(io::Error::other(URL)), Error::Connect("other error".to_owned()))]
    #[case::url(
        WebSocketError::Url(UrlError::UnableToConnect(URL.to_owned())),
        Error::Connect("not a ws:// address".to_owned())
    )]
    #[case::protocol(
        WebSocketError::Protocol(ProtocolError::SecWebSocketAcceptKeyMismatch),
        Error::Connect(r#"the WebSocket upgrade failed: Key mismatch in "Sec-WebSocket-Accept" header"#.to_owned())
    )]
    #[case::closed(WebSocketError::ConnectionClosed, Error::Connect("the WebSocket upgrade failed".to_owned()))]
    fn translates_each_failure(#[case] error: WebSocketError, #[case] expected: Error) {
        assert_eq!(connect_error(error), expected);
    }
}

mod secrecy {
    use super::*;

    #[test]
    fn shows_neither_the_url_nor_the_token_of_the_transport() {
        let transport = BridgeTransport::new(URL.to_owned(), BridgeConfig::default());
        assert_eq!(format!("{transport:?}"), "BridgeTransport { config: BridgeConfig { connect_timeout: 3s }, .. }");
    }

    #[test]
    fn shows_only_the_timeout_of_the_config() {
        assert_eq!(format!("{:?}", BridgeConfig::default()), "BridgeConfig { connect_timeout: 3s }");
    }

    #[rstest]
    #[case::url(WebSocketError::Url(UrlError::UnableToConnect(URL.to_owned())))]
    #[case::io(WebSocketError::Io(io::Error::other(URL)))]
    #[case::refusal(refusal(503))]
    fn keeps_them_out_of_a_connection_error(#[case] error: WebSocketError) {
        let error = connect_error(error);
        for shown in [error.to_string(), format!("{error:?}")] {
            assert!(!shown.contains("secret") && !shown.contains("bridge.example"), "{shown}");
        }
    }
}
