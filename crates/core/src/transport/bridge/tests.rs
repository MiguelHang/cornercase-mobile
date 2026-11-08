use std::io;
use std::iter;
use std::time::Duration;

use futures_util::future::{self, Either};
use futures_util::{SinkExt, StreamExt, stream};
use rstest::rstest;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::error::{ProtocolError, UrlError};
use tokio_tungstenite::tungstenite::handshake::server::{Callback, ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::{accept_async, accept_hdr_async};

use super::*;
use crate::protocol::{BridgeMessage, ClientMessage};
use crate::transport::{Out, listen};

const URL: &str = "ws://bridge.example:7777/?token=secret";
const UNRESOLVED: &str = "the address could not be resolved or reached";

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
    fn bears_the_bridges_silence_by_default() {
        assert_eq!(BridgeConfig::default().silence_timeout, Duration::from_secs(45));
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
        let config = BridgeConfig { connect_timeout: Duration::from_millis(100), ..BridgeConfig::default() };
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
        let transport = BridgeTransport::new(URL.to_owned(), BridgeConfig::default());
        assert_eq!(
            format!("{transport:?}"),
            "BridgeTransport { config: BridgeConfig { connect_timeout: 3s, silence_timeout: 45s }, .. }"
        );
    }

    #[test]
    fn shows_only_the_timeouts_of_the_config() {
        assert_eq!(
            format!("{:?}", BridgeConfig::default()),
            "BridgeConfig { connect_timeout: 3s, silence_timeout: 45s }"
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

    type Bridge = WebSocketStream<TcpStream>;

    fn argv() -> Vec<String> {
        vec!["send".to_owned(), "-".to_owned()]
    }

    fn sent(message: &ClientMessage) -> String {
        serde_json::to_string(message).expect("a message")
    }

    async fn heard(bridge: &mut Bridge) -> Vec<String> {
        let mut heard = Vec::new();
        while let Some(Ok(message)) = bridge.next().await {
            let Message::Text(text) = message else { continue };
            heard.push(text.as_str().to_owned());
            if text == sent(&ClientMessage::Eof) {
                break;
            }
        }
        heard
    }

    async fn say(bridge: &mut Bridge, messages: &[BridgeMessage]) {
        for message in messages {
            bridge.send(Message::text(serde_json::to_string(message).expect("a message"))).await.expect("sent");
        }
    }

    async fn closed_cleanly(bridge: &mut Bridge) -> bool {
        let mut closed = false;
        while let Some(message) = bridge.next().await {
            match message {
                Ok(Message::Close(_)) => closed = true,
                Ok(_) => {}
                Err(_) => return false,
            }
        }
        closed
    }

    async fn bridge<S, F>(config: BridgeConfig, script: S) -> (BridgeTransport, tokio::task::JoinHandle<F::Output>)
    where
        S: FnOnce(Bridge) -> F + Send + 'static,
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (url, listener) = listener().await;
        let script = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("a connection");
            script(accept_async(stream).await.expect("an upgrade")).await
        });
        (BridgeTransport::new(url, config), script)
    }

    async fn answer(messages: Vec<BridgeMessage>) -> (Result<Exit, Error>, Vec<Out>, (Vec<String>, bool)) {
        let (transport, script) = bridge(BridgeConfig::default(), async move |mut bridge| {
            let heard = heard(&mut bridge).await;
            say(&mut bridge, &messages).await;
            (heard, closed_cleanly(&mut bridge).await)
        })
        .await;
        let (out, mut receiver) = listen(Some);
        let stdin = stream::iter([Stdin::Text("one".to_owned()), Stdin::Bytes(vec![0, 255])]).boxed();
        let ended = transport.run(argv(), stdin, out).await;
        let outs = iter::from_fn(|| receiver.try_recv().ok()).collect();
        (ended, outs, script.await.expect("the bridge's script"))
    }

    #[tokio::test]
    async fn sends_the_argv_then_the_input_then_its_end() {
        let (_, _, (heard, _)) = answer(vec![BridgeMessage::Exit(0)]).await;
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
            BridgeMessage::Stdout("a".to_owned()),
            BridgeMessage::Stderr("b".to_owned()),
            BridgeMessage::Stdout("c".to_owned()),
            BridgeMessage::Stderr("d".to_owned()),
        ];
        let (ended, outs, _) = answer(lines.into_iter().chain([BridgeMessage::Exit(3)]).collect()).await;
        assert_eq!(ended, Ok(Exit { code: 3, stderr: "b\nd".to_owned() }));
        let expected = [Out::Stdout("a".to_owned()), Out::Stderr("b".to_owned())];
        let expected = expected.into_iter().chain([Out::Stdout("c".to_owned()), Out::Stderr("d".to_owned())]);
        assert_eq!(outs, expected.collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn closes_after_the_exit_and_ignores_what_follows_it() {
        let (ended, outs, (_, closed)) =
            answer(vec![BridgeMessage::Exit(0), BridgeMessage::Stdout("late".to_owned())]).await;
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
        let (transport, script) = bridge(BridgeConfig::default(), async move |mut bridge| {
            heard(&mut bridge).await;
            say(&mut bridge, &[BridgeMessage::Stdout("a".to_owned())]).await;
            if let Some(code) = code {
                bridge.close(Some(CloseFrame { code, reason: "".into() })).await.expect("closed");
            }
        })
        .await;
        let (out, mut receiver) = listen(Some);
        assert_eq!(transport.run(argv(), stream::empty().boxed(), out).await, Err(Error::Dropped));
        assert_eq!(receiver.try_recv().ok(), Some(Out::Stdout("a".to_owned())));
        script.await.expect("the bridge's script");
    }

    #[rstest]
    #[case::binary(Message::binary(b"{}".to_vec()))]
    #[case::not_the_bridges(Message::text(r#"{"stdin":"x"}"#))]
    #[tokio::test]
    async fn refuses_what_the_bridge_never_sends(#[case] message: Message) {
        let (transport, script) = bridge(BridgeConfig::default(), async move |mut bridge| {
            heard(&mut bridge).await;
            bridge.send(message).await.expect("sent");
            closed_cleanly(&mut bridge).await
        })
        .await;
        let (out, _receiver) = listen(Some);
        let ended = transport.run(argv(), stream::empty().boxed(), out).await;
        assert!(matches!(ended, Err(Error::Protocol(_))), "{ended:?}");
        assert!(script.await.expect("the bridge's script"), "the client did not close cleanly");
    }

    fn silent_after(silence: Duration) -> BridgeConfig {
        BridgeConfig { silence_timeout: silence, ..BridgeConfig::default() }
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
        assert!(script.await.expect("the bridge's script"), "the client did not close cleanly");
    }

    #[tokio::test]
    async fn bears_a_long_command_while_the_bridge_pings() {
        let silence = Duration::from_millis(300);
        let (transport, script) = bridge(silent_after(silence), async move |mut bridge| {
            heard(&mut bridge).await;
            for _ in 0..10 {
                tokio::time::sleep(silence / 6).await;
                bridge.send(Message::Ping(Vec::new().into())).await.expect("pinged");
            }
            say(&mut bridge, &[BridgeMessage::Exit(0)]).await;
            closed_cleanly(&mut bridge).await
        })
        .await;
        let (out, _receiver) = listen(Some);
        let ended = transport.run(argv(), stream::empty().boxed(), out).await;
        assert_eq!(ended, Ok(Exit { code: 0, stderr: String::new() }));
        assert!(script.await.expect("the bridge's script"), "the client did not close cleanly");
    }

    #[tokio::test]
    async fn closes_when_nobody_listens_to_the_output() {
        let (transport, script) = bridge(BridgeConfig::default(), async move |mut bridge| {
            heard(&mut bridge).await;
            say(&mut bridge, &[BridgeMessage::Stdout("a".to_owned())]).await;
            closed_cleanly(&mut bridge).await
        })
        .await;
        let (out, receiver) = listen(Some);
        drop(receiver);
        assert_eq!(transport.run(argv(), stream::empty().boxed(), out).await, Err(Error::OutputClosed));
        assert!(script.await.expect("the bridge's script"), "the client did not close cleanly");
    }

    #[tokio::test]
    async fn lets_go_of_the_connection_once_dropped() {
        let (argv_came, came) = oneshot::channel();
        let (transport, script) = bridge(BridgeConfig::default(), async move |mut bridge| {
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
        tokio::time::timeout(Duration::from_secs(5), script)
            .await
            .expect("the bridge still has the connection")
            .expect("the bridge's script");
    }
}
