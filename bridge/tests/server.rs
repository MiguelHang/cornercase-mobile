use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rstest::rstest;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::{Error, Message};

use cornercase_bridge::server::{self, Config};
use cornercase_bridge::test_util::{TempDir, fake_cornercase, in_time};

const TOKEN: &str = "s3cret";
const RIGHT: &str = "/?token=s3cret";
const WITNESS: &str = "touch \"${0%/*}/ran\"";
const USAGE: &str = r#"{"exit":2}"#;

struct Served {
    address: SocketAddr,
    _stop: oneshot::Sender<()>,
}

impl Served {
    async fn start(change: impl FnOnce(&mut Config)) -> Self {
        let mut config = Config::new(TOKEN.to_owned(), PathBuf::from("/nonexistent/cornercase"));
        change(&mut config);
        let address = "127.0.0.1:0".parse().expect("an address");
        let server = server::bind(address, config).await.expect("the server binds");
        let address = server.local_addr().expect("the server's address");
        let (stop, stopped) = oneshot::channel::<()>();
        tokio::spawn(server.run(async {
            let _ = stopped.await;
        }));
        Self { address, _stop: stop }
    }

    fn url(&self, target: &str) -> String {
        format!("ws://{}{target}", self.address)
    }

    async fn upgrade(&self, target: &str) -> StatusCode {
        match in_time(connect_async(self.url(target))).await {
            Ok((_, response)) => response.status(),
            Err(Error::Http(response)) => response.status(),
            Err(error) => panic!("the handshake ends with a status, not {error}"),
        }
    }

    async fn silent(&self) -> TcpStream {
        TcpStream::connect(self.address).await.expect("a connection")
    }

    async fn exchange(&self, sent: Vec<Message>) -> Exchange {
        let (mut socket, _) = in_time(connect_async(self.url(RIGHT))).await.expect("the upgrade");
        for message in sent {
            socket.send(message).await.expect("the message is sent");
        }
        let mut exchange = Exchange::default();
        while let Some(message) = in_time(socket.next()).await {
            match message.expect("a message") {
                Message::Text(text) => exchange.texts.push(text.as_str().to_owned()),
                Message::Close(frame) => exchange.close = frame.map(|frame| frame.code),
                _ => {}
            }
        }
        exchange
    }
}

#[derive(Debug, Default, PartialEq)]
struct Exchange {
    texts: Vec<String>,
    close: Option<CloseCode>,
}

impl Exchange {
    fn of(texts: &[&str]) -> Self {
        Self { texts: texts.iter().map(|&text| text.to_owned()).collect(), close: Some(CloseCode::Normal) }
    }
}

fn stderr(line: &str) -> String {
    format!(r#"{{"stderr":"{line}"}}"#)
}

fn ran(dir: &Path) -> bool {
    dir.join("ran").exists()
}

async fn closed(mut stream: TcpStream) {
    let mut read = Vec::new();
    assert_eq!(in_time(stream.read_to_end(&mut read)).await.expect("it reads to the end"), 0);
}

mod handshake {
    use super::*;

    #[rstest]
    #[case::no_token("/", StatusCode::UNAUTHORIZED)]
    #[case::a_wrong_token("/?token=wrong", StatusCode::UNAUTHORIZED)]
    #[case::an_empty_token("/?token=", StatusCode::UNAUTHORIZED)]
    #[case::the_token_twice("/?token=s3cret&token=s3cret", StatusCode::UNAUTHORIZED)]
    #[case::another_path("/x?token=s3cret", StatusCode::NOT_FOUND)]
    #[tokio::test]
    async fn refuses_without_upgrading(#[case] target: &str, #[case] status: StatusCode) {
        let served = Served::start(|_| {}).await;
        assert_eq!(served.upgrade(target).await, status);
    }

    #[tokio::test]
    async fn upgrades_with_the_right_token() {
        let served = Served::start(|_| {}).await;
        assert_eq!(served.upgrade(RIGHT).await, StatusCode::SWITCHING_PROTOCOLS);
    }

    #[tokio::test]
    async fn drops_a_connection_that_sends_no_request_in_time() {
        let served = Served::start(|config| config.handshake = Duration::from_millis(50)).await;
        closed(served.silent().await).await;
    }
}

mod connections {
    use super::*;

    #[tokio::test]
    async fn answers_busy_past_the_limit() {
        let served = Served::start(|config| config.connections = 1).await;
        let _held = served.silent().await;
        assert_eq!(served.upgrade(RIGHT).await, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn frees_a_place_once_a_connection_ends() {
        let served = Served::start(|config| {
            config.connections = 1;
            config.handshake = Duration::from_millis(50);
        })
        .await;
        closed(served.silent().await).await;
        assert_eq!(served.upgrade(RIGHT).await, StatusCode::SWITCHING_PROTOCOLS);
    }
}

mod opening {
    use super::*;

    fn witness(dir: &TempDir) -> impl FnOnce(&mut Config) {
        let binary = fake_cornercase(dir.path(), &format!("{WITNESS}\necho hi"));
        move |config| {
            config.binary = binary;
            config.hello = Duration::from_millis(100);
        }
    }

    #[rstest]
    #[case::an_argv_the_allowlist_refuses(
        vec![Message::text(r#"{"argv":["kill-server"]}"#)],
        "`kill-server` is not allowed through the bridge"
    )]
    #[case::stdin_first(vec![Message::text(r#"{"stdin":"x"}"#)], "the first message must be `argv`")]
    #[case::a_binary_frame(vec![Message::binary(vec![1, 2])], "the first message must be text, not binary")]
    #[case::bad_json(vec![Message::text("{")], "bad message: the JSON ends too early")]
    #[case::nothing_in_time(Vec::new(), "no `argv` within 100ms")]
    #[tokio::test]
    async fn answers_a_refusal_and_runs_nothing(#[case] sent: Vec<Message>, #[case] reason: &str) {
        let dir = TempDir::new();
        let served = Served::start(witness(&dir)).await;
        assert_eq!(served.exchange(sent).await, Exchange::of(&[&stderr(reason), USAGE]));
        assert!(!ran(dir.path()));
    }

    #[tokio::test]
    async fn answers_a_refusal_when_the_binary_is_gone() {
        let dir = TempDir::new();
        let served = Served::start(witness(&dir)).await;
        std::fs::remove_file(dir.path().join("cornercase")).expect("remove the binary");
        let exchange = served.exchange(vec![Message::text(r#"{"argv":["status"]}"#)]).await;
        assert!(exchange.texts[0].starts_with(r#"{"stderr":"could not run cornercase: "#));
        assert_eq!(exchange, Exchange::of(&[&exchange.texts[0], USAGE]));
    }

    #[tokio::test]
    async fn runs_an_allowed_command() {
        let dir = TempDir::new();
        let served = Served::start(witness(&dir)).await;
        let exchange = served.exchange(vec![Message::text(r#"{"argv":["status"]}"#)]).await;
        assert_eq!(exchange, Exchange::of(&[r#"{"stdout":"hi"}"#, r#"{"exit":0}"#]));
        assert!(ran(dir.path()));
    }
}
