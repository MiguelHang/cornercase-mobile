use std::io;
use std::iter;
use std::time::Duration;

use futures_util::future::{self, Either};
use futures_util::{SinkExt, StreamExt, stream};
use rstest::rstest;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::error::{ProtocolError, UrlError};
use tokio_tungstenite::tungstenite::handshake::server::{Callback, ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use super::*;
use crate::protocol::{ClientMessage, ServerMessage};
use crate::transport::fake::cli::{CLI, Peer, closed_cleanly, heard, listening, say, serve};
use crate::transport::{Out, listen};

const URL: &str = "ws://bridge.example:7777/?token=secret";
const UNRESOLVED: &str = "the address could not be resolved or reached";
const BRIDGE: &str = "/?token=secret";
const SECRET: &str = "4f2a91c0.9c1e2a7b5d0f";

type Head = (String, Option<String>);

async fn within<T>(work: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), work).await.expect("it gets there within five seconds")
}

fn refusal(status: u16) -> WebSocketError {
    let response = http::Response::builder().status(status).body(None).expect("a response");
    WebSocketError::Http(Box::new(response))
}

async fn listener() -> (String, TcpListener) {
    listening(BRIDGE).await
}

struct Answer(Option<u16>, oneshot::Sender<Head>);

impl Callback for Answer {
    fn on_request(self, request: &Request, response: Response) -> Result<Response, ErrorResponse> {
        let authorization = request.headers().get(AUTHORIZATION).and_then(|value| value.to_str().ok());
        let _ = self.1.send((request.uri().to_string(), authorization.map(str::to_owned)));
        match self.0 {
            None => Ok(response),
            Some(status) => Err(http::Response::builder().status(status).body(None).expect("a refusal")),
        }
    }
}

async fn upgrade(listener: TcpListener, answer: Option<u16>, head: oneshot::Sender<Head>) {
    let (stream, _) = listener.accept().await.expect("a connection");
    let _ = accept_hdr_async(stream, Answer(answer, head)).await;
}

async fn connect_with(path: &str, bearer: Option<&str>, answer: Option<u16>) -> (Result<Socket, Error>, Head) {
    let (url, listener) = listening(path).await;
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(upgrade(listener, answer, sender));
    let transport = WebSocketTransport::new(url, SocketConfig::default());
    let transport = match bearer {
        Some(secret) => transport.bearing(secret.to_owned()),
        None => transport,
    };
    let connected = transport.connect().await;
    (connected, within(receiver).await.expect("the head of the upgrade"))
}

async fn connect_to(answer: Option<u16>) -> Result<Socket, Error> {
    connect_with(BRIDGE, None, answer).await.0
}

fn cli(url: String) -> WebSocketTransport {
    WebSocketTransport::new(url, SocketConfig::default()).bearing(SECRET.to_owned())
}

mod config {
    use super::*;

    #[test]
    fn waits_three_seconds_to_connect_by_default() {
        assert_eq!(SocketConfig::default().connect_timeout, Duration::from_secs(3));
    }

    #[test]
    fn bears_the_server_s_silence_for_45_seconds_by_default() {
        assert_eq!(SocketConfig::default().silence_timeout, Duration::from_secs(45));
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
    #[case::forbidden(403, Error::Rejected { status: 403 })]
    #[case::not_found(404, Error::Rejected { status: 404 })]
    #[case::unavailable(503, Error::Rejected { status: 503 })]
    #[tokio::test]
    async fn tells_a_refused_token_from_another_refusal(#[case] status: u16, #[case] expected: Error) {
        assert_eq!(connect_to(Some(status)).await.err(), Some(expected));
    }

    #[rstest]
    #[case::forbidden(403, Error::NotPaired)]
    #[case::unavailable(503, Error::Rejected { status: 503 })]
    #[tokio::test]
    async fn tells_a_forgotten_secret_from_another_refusal(#[case] status: u16, #[case] expected: Error) {
        let (connected, _) = connect_with(CLI, Some(SECRET), Some(status)).await;
        assert_eq!(connected.err(), Some(expected));
    }

    #[tokio::test]
    async fn takes_the_secret_in_the_authorization_header() {
        let (connected, head) = connect_with(CLI, Some(SECRET), None).await;
        assert!(connected.is_ok(), "the upgrade failed");
        assert_eq!(head, (CLI.to_owned(), Some(format!("Bearer {SECRET}"))));
    }

    #[tokio::test]
    async fn sends_no_authorization_to_the_bridge() {
        let (_, head) = connect_with(BRIDGE, None, None).await;
        assert_eq!(head, (BRIDGE.to_owned(), None));
    }

    #[tokio::test]
    async fn refuses_a_secret_that_cannot_go_in_a_header() {
        let (url, _listener) = listening(CLI).await;
        let transport = WebSocketTransport::new(url, SocketConfig::default()).bearing("a\nb".to_owned());
        let refused = Some(Error::Secrets("the saved secret cannot go in a header".to_owned()));
        assert_eq!(transport.connect().await.err(), refused);
    }

    #[tokio::test]
    async fn gives_up_when_the_upgrade_does_not_come_in_time() {
        let (url, _listener) = listener().await;
        let config = SocketConfig { connect_timeout: Duration::from_millis(100), ..SocketConfig::default() };
        assert_eq!(WebSocketTransport::new(url, config).connect().await.err(), Some(Error::Timeout));
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
    #[case::other_io(WebSocketError::Io(io::Error::other(URL)), Error::Connect(UNRESOLVED.to_owned()))]
    #[case::no_address(
        WebSocketError::Io(io::Error::new(io::ErrorKind::InvalidInput, "could not resolve to any address")),
        Error::Connect(UNRESOLVED.to_owned())
    )]
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
        let transport = WebSocketTransport::new(URL.to_owned(), SocketConfig::default());
        assert_eq!(
            format!("{transport:?}"),
            "WebSocketTransport { config: SocketConfig { connect_timeout: 3s, silence_timeout: 45s }, .. }"
        );
    }

    #[test]
    fn shows_nothing_of_the_secret_it_bears() {
        let shown = format!("{:?}", cli(URL.to_owned()));
        assert!(!shown.contains(SECRET) && !shown.contains("4f2a91c0"), "{shown}");
    }

    #[test]
    fn marks_the_authorization_header_as_sensitive() {
        let request = cli(URL.to_owned()).request().expect("a request");
        assert!(request.headers().get(AUTHORIZATION).is_some_and(HeaderValue::is_sensitive));
    }

    #[test]
    fn shows_only_the_timeouts_of_the_config() {
        assert_eq!(
            format!("{:?}", SocketConfig::default()),
            "SocketConfig { connect_timeout: 3s, silence_timeout: 45s }"
        );
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

mod run {
    use super::*;

    fn argv() -> Vec<String> {
        vec!["send".to_owned(), "-".to_owned()]
    }

    fn sent(message: &ClientMessage) -> String {
        serde_json::to_string(message).expect("a message")
    }

    async fn bridge<S, F>(config: SocketConfig, script: S) -> (WebSocketTransport, tokio::task::JoinHandle<F::Output>)
    where
        S: FnOnce(Peer) -> F + Send + 'static,
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (url, listener) = listening(CLI).await;
        (WebSocketTransport::new(url, config).bearing(SECRET.to_owned()), serve(listener, SECRET, script))
    }

    async fn answer(messages: Vec<ServerMessage>) -> (Result<Exit, Error>, Vec<Out>, (Vec<String>, bool)) {
        let (transport, script) = bridge(SocketConfig::default(), async move |mut bridge| {
            let heard = heard(&mut bridge).await;
            say(&mut bridge, &messages).await;
            (heard, closed_cleanly(&mut bridge).await)
        })
        .await;
        let (out, mut receiver) = listen(Some);
        let stdin = stream::iter([Stdin::Text("one".to_owned()), Stdin::Bytes(vec![0, 255])]).boxed();
        let ended = transport.run(argv(), stdin, out).await;
        let outs = iter::from_fn(|| receiver.try_recv().ok()).collect();
        (ended, outs, within(script).await.expect("the bridge's script"))
    }

    #[tokio::test]
    async fn sends_the_argv_then_the_input_then_its_end() {
        let (_, _, (heard, _)) = answer(vec![ServerMessage::Exit(0)]).await;
        let expected = [
            ClientMessage::Argv(argv()),
            ClientMessage::Stdin("one".to_owned()),
            ClientMessage::StdinBase64(vec![0, 255]),
            ClientMessage::Eof,
        ];
        assert_eq!(heard, expected.iter().map(sent).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn hands_the_output_on_in_order_and_exits_with_the_code() {
        let lines = [
            ServerMessage::Stdout("a".to_owned()),
            ServerMessage::Stderr("b".to_owned()),
            ServerMessage::Stdout("c".to_owned()),
            ServerMessage::Stderr("d".to_owned()),
        ];
        let (ended, outs, _) = answer(lines.into_iter().chain([ServerMessage::Exit(3)]).collect()).await;
        assert_eq!(ended, Ok(Exit { code: 3, stderr: "b\nd".to_owned() }));
        let expected = [Out::Stdout("a".to_owned()), Out::Stderr("b".to_owned())];
        let expected = expected.into_iter().chain([Out::Stdout("c".to_owned()), Out::Stderr("d".to_owned())]);
        assert_eq!(outs, expected.collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn closes_after_the_exit_and_ignores_what_follows_it() {
        let (ended, outs, (_, closed)) =
            answer(vec![ServerMessage::Exit(0), ServerMessage::Stdout("late".to_owned())]).await;
        assert_eq!(ended, Ok(Exit { code: 0, stderr: String::new() }));
        assert_eq!(outs, Vec::new());
        assert!(closed, "the client did not close cleanly");
    }

    #[rstest]
    #[case::going_away(Some(CloseCode::Away))]
    #[case::normal(Some(CloseCode::Normal))]
    #[case::broken(None)]
    #[tokio::test]
    async fn drops_when_the_bridge_goes_without_an_exit(#[case] code: Option<CloseCode>) {
        let (transport, script) = bridge(SocketConfig::default(), async move |mut bridge| {
            heard(&mut bridge).await;
            say(&mut bridge, &[ServerMessage::Stdout("a".to_owned())]).await;
            if let Some(code) = code {
                bridge.close(Some(CloseFrame { code, reason: "".into() })).await.expect("closed");
            }
        })
        .await;
        let (out, mut receiver) = listen(Some);
        assert_eq!(transport.run(argv(), stream::empty().boxed(), out).await, Err(Error::Dropped));
        assert_eq!(receiver.try_recv().ok(), Some(Out::Stdout("a".to_owned())));
        within(script).await.expect("the bridge's script");
    }

    #[rstest]
    #[case::binary(Message::binary(b"{}".to_vec()))]
    #[case::not_the_bridges(Message::text(r#"{"stdin":"x"}"#))]
    #[tokio::test]
    async fn refuses_what_the_bridge_never_sends(#[case] message: Message) {
        let (transport, script) = bridge(SocketConfig::default(), async move |mut bridge| {
            heard(&mut bridge).await;
            bridge.send(message).await.expect("sent");
            closed_cleanly(&mut bridge).await
        })
        .await;
        let (out, _receiver) = listen(Some);
        let ended = transport.run(argv(), stream::empty().boxed(), out).await;
        assert!(matches!(ended, Err(Error::Protocol(_))), "{ended:?}");
        assert!(within(script).await.expect("the bridge's script"), "the client did not close cleanly");
    }

    fn silent_after(silence: Duration) -> SocketConfig {
        SocketConfig { silence_timeout: silence, ..SocketConfig::default() }
    }

    #[tokio::test]
    async fn drops_when_the_bridge_falls_silent() {
        let silence = Duration::from_millis(100);
        let (transport, script) = bridge(silent_after(silence), async move |mut bridge| {
            heard(&mut bridge).await;
            closed_cleanly(&mut bridge).await
        })
        .await;
        let (out, _receiver) = listen(Some);
        let started = tokio::time::Instant::now();
        assert_eq!(transport.run(argv(), stream::empty().boxed(), out).await, Err(Error::Dropped));
        assert!(started.elapsed() >= silence, "it gave up after {:?}", started.elapsed());
        assert!(within(script).await.expect("the bridge's script"), "the client did not close cleanly");
    }

    async fn ping(bridge: &mut Peer, every: Duration, times: usize) {
        for _ in 0..times {
            tokio::time::sleep(every).await;
            bridge.send(Message::Ping(Vec::new().into())).await.expect("pinged");
        }
    }

    #[tokio::test]
    async fn bears_a_long_command_while_the_bridge_pings() {
        let silence = Duration::from_millis(300);
        let (transport, script) = bridge(silent_after(silence), async move |mut bridge| {
            heard(&mut bridge).await;
            ping(&mut bridge, silence / 6, 10).await;
            say(&mut bridge, &[ServerMessage::Exit(0)]).await;
            closed_cleanly(&mut bridge).await
        })
        .await;
        let (out, _receiver) = listen(Some);
        let ended = transport.run(argv(), stream::empty().boxed(), out).await;
        assert_eq!(ended, Ok(Exit { code: 0, stderr: String::new() }));
        assert!(within(script).await.expect("the bridge's script"), "the client did not close cleanly");
    }

    #[tokio::test]
    async fn closes_when_nobody_listens_to_the_output() {
        let (transport, script) = bridge(SocketConfig::default(), async move |mut bridge| {
            heard(&mut bridge).await;
            say(&mut bridge, &[ServerMessage::Stdout("a".to_owned())]).await;
            closed_cleanly(&mut bridge).await
        })
        .await;
        let (out, receiver) = listen(Some);
        drop(receiver);
        assert_eq!(transport.run(argv(), stream::empty().boxed(), out).await, Err(Error::OutputClosed));
        assert!(within(script).await.expect("the bridge's script"), "the client did not close cleanly");
    }

    #[tokio::test]
    async fn lets_go_of_the_connection_once_dropped() {
        let (argv_came, came) = oneshot::channel();
        let (transport, script) = bridge(SocketConfig::default(), async move |mut bridge| {
            bridge.next().await;
            argv_came.send(()).expect("the test waits");
            closed_cleanly(&mut bridge).await
        })
        .await;
        let (out, _receiver) = listen(Some);
        let running = transport.run(argv(), stream::pending().boxed(), out);
        let Either::Right((_, running)) = future::select(running, came).await else {
            panic!("the run ended by itself");
        };
        drop(running);
        within(script).await.expect("the bridge's script");
    }
}
