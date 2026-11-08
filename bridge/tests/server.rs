use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rstest::rstest;
use rustix::io::Errno;
use rustix::process::{Pid, test_kill_process};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::{Error, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use cornercase_bridge::server::{self, Config};
use cornercase_bridge::test_util::{TempDir, fake_cornercase, in_time, wait_until};

const TOKEN: &str = "s3cret";
const RIGHT: &str = "/?token=s3cret";
const WITNESS: &str = "touch \"${0%/*}/ran\"";
const USAGE: &str = r#"{"exit":2}"#;
const EXITED: &str = r#"{"exit":0}"#;
const ARGV: &str = r#"{"argv":["status"]}"#;
const EOF: &str = r#"{"eof":true}"#;
const READY: &str = r#"{"stdout":"ready"}"#;
const HUNG: &str = "echo $$ > \"${0%/*}/pid\"\necho ready\nexec sleep 30";

type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

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

    async fn connect(&self) -> Client {
        in_time(connect_async(self.url(RIGHT))).await.expect("the upgrade").0
    }

    async fn exchange(&self, sent: Vec<Message>) -> Exchange {
        let mut socket = self.connect().await;
        send(&mut socket, sent).await;
        Exchange::read(socket).await
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

    async fn read(mut socket: Client) -> Self {
        let mut exchange = Self::default();
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

async fn send(socket: &mut Client, messages: Vec<Message>) {
    for message in messages {
        socket.send(message).await.expect("the message is sent");
    }
}

fn faking(dir: &TempDir, script: &str) -> impl FnOnce(&mut Config) + use<> {
    let binary = fake_cornercase(dir.path(), script);
    move |config| config.binary = binary
}

fn stdin(text: &str) -> Message {
    Message::text(format!(r#"{{"stdin":"{text}"}}"#))
}

fn pieces(encoded: &str) -> Message {
    Message::text(format!(r#"{{"stdin_base64":"{encoded}"}}"#))
}

fn stderr(line: &str) -> String {
    format!(r#"{{"stderr":"{line}"}}"#)
}

fn ran(dir: &Path) -> bool {
    dir.join("ran").exists()
}

fn reaped(pid: i32) -> bool {
    Pid::from_raw(pid).is_some_and(|pid| matches!(test_kill_process(pid), Err(Errno::SRCH)))
}

fn number_in(dir: &Path, name: &str) -> Option<i32> {
    std::fs::read_to_string(dir.join(name)).ok()?.trim().parse().ok()
}

async fn next_text(socket: &mut Client) -> String {
    loop {
        match in_time(socket.next()).await {
            Some(Ok(Message::Text(text))) => return text.as_str().to_owned(),
            Some(Ok(Message::Ping(_))) => {}
            other => panic!("a text message, not {other:?}"),
        }
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

mod opening {
    use super::*;

    fn witness(dir: &TempDir) -> impl FnOnce(&mut Config) {
        let fake = faking(dir, &format!("{WITNESS}\necho hi"));
        move |config| {
            fake(config);
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
        let exchange = served.exchange(vec![Message::text(ARGV)]).await;
        assert!(exchange.texts[0].starts_with(r#"{"stderr":"could not run cornercase: "#));
        assert_eq!(exchange, Exchange::of(&[&exchange.texts[0], USAGE]));
    }

    #[tokio::test]
    async fn runs_an_allowed_command() {
        let dir = TempDir::new();
        let served = Served::start(witness(&dir)).await;
        let exchange = served.exchange(vec![Message::text(ARGV)]).await;
        assert_eq!(exchange, Exchange::of(&[r#"{"stdout":"hi"}"#, EXITED]));
        assert!(ran(dir.path()));
    }
}

mod running {
    use super::*;

    const CAT: &str = "exec cat";

    #[rstest]
    #[case::to_stdout(CAT, vec![stdin("hi"), stdin("there"), Message::text(EOF)], &[
        r#"{"stdout":"hi"}"#, r#"{"stdout":"there"}"#, EXITED
    ])]
    #[case::to_stderr("exec cat >&2", vec![stdin("hi"), Message::text(EOF)], &[r#"{"stderr":"hi"}"#, EXITED])]
    #[case::only_eof(CAT, vec![Message::text(EOF)], &[EXITED])]
    #[case::stdin_base64_in_pieces(CAT, vec![pieces("aGVs"), pieces("bG8K"), Message::text(EOF)], &[
        r#"{"stdout":"hello"}"#, EXITED
    ])]
    #[case::stdin_after_eof(CAT, vec![stdin("a"), Message::text(EOF), stdin("b")], &[r#"{"stdout":"a"}"#, EXITED])]
    #[tokio::test]
    async fn sends_the_output_then_the_exit_then_closes(
        #[case] script: &str,
        #[case] input: Vec<Message>,
        #[case] texts: &[&str],
    ) {
        let dir = TempDir::new();
        let served = Served::start(faking(&dir, script)).await;
        let exchange = served.exchange([vec![Message::text(ARGV)], input].concat()).await;
        assert_eq!(exchange, Exchange::of(texts));
    }

    #[rstest]
    #[case::bad_json(Message::text("{"), "bad message: the JSON ends too early")]
    #[case::a_second_argv(Message::text(ARGV), "`argv` was already sent")]
    #[case::a_binary_frame(Message::binary(vec![1]), "a message must be text, not binary")]
    #[tokio::test]
    async fn ends_the_command_on_a_bad_message(#[case] message: Message, #[case] reason: &str) {
        let dir = TempDir::new();
        let served = Served::start(faking(&dir, HUNG)).await;
        let mut socket = served.connect().await;
        send(&mut socket, vec![Message::text(ARGV)]).await;
        assert_eq!(next_text(&mut socket).await, READY);
        send(&mut socket, vec![message]).await;
        assert_eq!(Exchange::read(socket).await, Exchange::of(&[&stderr(reason), USAGE]));
        assert!(reaped(number_in(dir.path(), "pid").expect("the fake wrote its pid")));
    }

    #[tokio::test]
    async fn keeps_each_connection_to_its_own_output() {
        let dir = TempDir::new();
        let served = Served::start(faking(&dir, CAT)).await;
        let mut one = served.connect().await;
        let mut two = served.connect().await;
        send(&mut one, vec![Message::text(ARGV), stdin("one")]).await;
        send(&mut two, vec![Message::text(ARGV), stdin("two")]).await;
        send(&mut one, vec![Message::text(EOF)]).await;
        send(&mut two, vec![Message::text(EOF)]).await;
        assert_eq!(Exchange::read(one).await, Exchange::of(&[r#"{"stdout":"one"}"#, EXITED]));
        assert_eq!(Exchange::read(two).await, Exchange::of(&[r#"{"stdout":"two"}"#, EXITED]));
    }
}

mod ending {
    use super::*;

    const GROUP: &str = "sleep 30 &\necho $! > \"${0%/*}/grandchild\"\necho ready\nwait";
    const FLOOD: &str = "i=0\nwhile :; do\n  i=$((i+1))\n  echo $i > \"${0%/*}/count.new\"\n  \
        mv \"${0%/*}/count.new\" \"${0%/*}/count\"\n  printf '%065536d\\n' 0\ndone";
    const QUIET: Duration = Duration::from_millis(300);

    fn heartbeat(ping: u64, silence: u64) -> impl Fn(&mut Config) {
        move |config| {
            config.ping = Duration::from_millis(ping);
            config.silence = Duration::from_millis(silence);
        }
    }

    async fn started(served: &Served) -> Client {
        let mut socket = served.connect().await;
        send(&mut socket, vec![Message::text(ARGV)]).await;
        assert_eq!(next_text(&mut socket).await, READY);
        socket
    }

    async fn settled(dir: &Path) -> i32 {
        let mut last = (None, Instant::now());
        wait_until(async || {
            let count = number_in(dir, "count");
            if count != last.0 {
                last = (count, Instant::now());
            }
            count.is_some() && last.1.elapsed() >= QUIET
        })
        .await;
        last.0.expect("the fake counts")
    }

    #[tokio::test]
    async fn kills_the_command_and_its_group_when_the_client_closes() {
        let dir = TempDir::new();
        let served = Served::start(faking(&dir, GROUP)).await;
        let mut socket = started(&served).await;
        let grandchild = number_in(dir.path(), "grandchild").expect("the fake wrote its child's pid");
        socket.close(None).await.expect("the client closes");
        wait_until(async || reaped(grandchild)).await;
    }

    #[tokio::test]
    async fn cuts_a_client_that_neither_reads_nor_answers() {
        let dir = TempDir::new();
        let fake = faking(&dir, HUNG);
        let served = Served::start(|config| {
            fake(config);
            heartbeat(20, 100)(config);
        })
        .await;
        let mut socket = started(&served).await;
        let pid = number_in(dir.path(), "pid").expect("the fake wrote its pid");
        wait_until(async || reaped(pid)).await;
        while let Some(Ok(_)) = in_time(socket.next()).await {}
    }

    #[tokio::test]
    async fn keeps_a_client_that_reads_past_the_silence() {
        let dir = TempDir::new();
        let fake = faking(&dir, HUNG);
        let served = Served::start(|config| {
            fake(config);
            heartbeat(20, 100)(config);
        })
        .await;
        let mut socket = started(&served).await;
        let mut pings = 0;
        while pings < 10 {
            pings += usize::from(matches!(in_time(socket.next()).await, Some(Ok(Message::Ping(_)))));
        }
        assert!(!reaped(number_in(dir.path(), "pid").expect("the fake wrote its pid")));
    }

    #[tokio::test]
    async fn holds_the_command_while_the_client_does_not_read() {
        let dir = TempDir::new();
        let served = Served::start(faking(&dir, FLOOD)).await;
        let mut socket = served.connect().await;
        send(&mut socket, vec![Message::text(ARGV)]).await;
        let held = settled(dir.path()).await;
        wait_until(async || {
            in_time(socket.next()).await;
            number_in(dir.path(), "count").is_some_and(|count| count > held)
        })
        .await;
    }
}
