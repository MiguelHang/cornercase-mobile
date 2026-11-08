use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use rstest::rstest;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Error;
use tokio_tungstenite::tungstenite::http::StatusCode;

use cornercase_bridge::server::{self, Config};
use cornercase_bridge::test_util::in_time;

const TOKEN: &str = "s3cret";
const RIGHT: &str = "/?token=s3cret";

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
