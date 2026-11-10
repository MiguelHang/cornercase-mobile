use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rstest::rstest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::{Error, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use cornercase_bridge::server::{Config, Server};
use cornercase_bridge::test_util::{
    HUNG, Served, TOKEN, TempDir, fake_cornercase, in_time, number_in, reaped, wait_until,
};

const RIGHT: &str = "/?token=s3cret";
const WITNESS: &str = "touch \"${0%/*}/ran\"";
const USAGE: &str = r#"{"exit":2}"#;
const EXITED: &str = r#"{"exit":0}"#;
const ARGV: &str = r#"{"argv":["status"]}"#;
const EOF: &str = r#"{"eof":true}"#;
const READY: &str = r#"{"stdout":"ready"}"#;

type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

trait Reach {
    async fn upgrade(&self, target: &str) -> StatusCode;
    async fn silent(&self) -> TcpStream;
    async fn connect(&self) -> Client;
    async fn exchange(&self, sent: Vec<Message>) -> Exchange;
}

impl Reach for Served {
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

    fn away() -> Self {
        Self { texts: Vec::new(), close: Some(CloseCode::Away) }
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

async fn next_text(socket: &mut Client) -> String {
    loop {
        match in_time(socket.next()).await {
            Some(Ok(Message::Text(text))) => return text.as_str().to_owned(),
            Some(Ok(Message::Ping(_))) => {}
            other => panic!("a text message, not {other:?}"),
        }
    }
}

async fn started(served: &Served) -> Client {
    let mut socket = served.connect().await;
    send(&mut socket, vec![Message::text(ARGV)]).await;
    assert_eq!(next_text(&mut socket).await, READY);
    socket
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

    async fn full(dir: &TempDir) -> (Served, Client) {
        let served = Served::start(|config| {
            faking(dir, HUNG)(config);
            config.connections = 1;
        })
        .await;
        let held = started(&served).await;
        (served, held)
    }

    #[rstest]
    #[case::no_token("/", StatusCode::UNAUTHORIZED)]
    #[case::a_wrong_token("/?token=wrong", StatusCode::UNAUTHORIZED)]
    #[case::another_path("/x?token=s3cret", StatusCode::NOT_FOUND)]
    #[case::the_right_token(RIGHT, StatusCode::SERVICE_UNAVAILABLE)]
    #[tokio::test]
    async fn answers_busy_past_the_limit_only_to_the_right_token(#[case] target: &str, #[case] status: StatusCode) {
        let dir = TempDir::new();
        let (served, _held) = full(&dir).await;
        assert_eq!(served.upgrade(target).await, status);
    }

    #[tokio::test]
    async fn takes_no_place_for_a_connection_that_has_not_upgraded() {
        let served = Served::start(|config| config.connections = 1).await;
        let _silent = served.silent().await;
        assert_eq!(served.upgrade(RIGHT).await, StatusCode::SWITCHING_PROTOCOLS);
    }

    #[tokio::test]
    async fn frees_a_place_once_a_connection_ends() {
        let dir = TempDir::new();
        let (served, mut held) = full(&dir).await;
        held.close(None).await.expect("the client closes");
        wait_until(async || served.upgrade(RIGHT).await == StatusCode::SWITCHING_PROTOCOLS).await;
    }
}

mod handshakes {
    use super::*;

    async fn crowded(within: Duration) -> (Served, [TcpStream; 2]) {
        let served = Served::start(|config| {
            config.handshakes = 2;
            config.handshake = within;
        })
        .await;
        let silent = [served.silent().await, served.silent().await];
        (served, silent)
    }

    #[tokio::test]
    async fn closes_a_connection_past_the_limit_without_answering() {
        let (served, _silent) = crowded(Duration::from_secs(60)).await;
        closed(served.silent().await).await;
    }

    #[tokio::test]
    async fn lets_a_client_in_once_the_silent_ones_run_out_of_time() {
        let (served, [first, second]) = crowded(Duration::from_millis(50)).await;
        closed(first).await;
        closed(second).await;
        assert_eq!(served.upgrade(RIGHT).await, StatusCode::SWITCHING_PROTOCOLS);
    }
}

mod listeners {
    use super::*;

    async fn listener() -> TcpListener {
        TcpListener::bind("127.0.0.1:0").await.expect("a listener")
    }

    #[tokio::test]
    async fn runs_a_command_through_each_and_closes_them_all_on_stopping() {
        let dir = TempDir::new();
        let mut config = Config::new(TOKEN.to_owned(), PathBuf::new());
        faking(&dir, "echo hi")(&mut config);
        let listening = Server::from_listeners(vec![listener().await, listener().await], config);
        let addresses = listening.local_addrs().expect("the addresses");
        let mut served = Served::serving(listening);
        for &address in &addresses {
            served.address = address;
            let exchange = served.exchange(vec![Message::text(ARGV)]).await;
            assert_eq!(exchange, Exchange::of(&[r#"{"stdout":"hi"}"#, EXITED]));
        }
        served.stop().await;
        for address in addresses {
            assert!(TcpStream::connect(address).await.is_err(), "{address} still accepts");
        }
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

mod routes {
    use super::*;

    const ASKED: &str = r#"{"argv":["--bridge-routes"]}"#;
    const ANSWER: &str = r#"{"stdout":"{\"routes\":[{\"host\":\"192.168.1.20\",\"port\":8765},{\"host\":\"fd7a:115c:a1e0::1\",\"port\":8765}]}"}"#;

    fn listening(dir: &TempDir) -> impl FnOnce(&mut Config) {
        let fake = faking(dir, WITNESS);
        move |config| {
            fake(config);
            config.addresses = ["192.168.1.20:8765", "[fd7a:115c:a1e0::1]:8765"]
                .map(|address| address.parse().expect("an address"))
                .to_vec();
        }
    }

    #[rstest]
    #[case::alone(vec![Message::text(ASKED)])]
    #[case::then_eof(vec![Message::text(ASKED), Message::text(EOF)])]
    #[tokio::test]
    async fn answers_the_addresses_it_listens_on_without_running_cornercase(#[case] sent: Vec<Message>) {
        let dir = TempDir::new();
        let served = Served::start(listening(&dir)).await;
        assert_eq!(served.exchange(sent).await, Exchange::of(&[ANSWER, EXITED]));
        assert!(!ran(dir.path()));
    }

    #[tokio::test]
    async fn never_answers_the_token() {
        let dir = TempDir::new();
        let served = Served::start(listening(&dir)).await;
        let exchange = served.exchange(vec![Message::text(ASKED)]).await;
        assert!(exchange.texts.iter().all(|text| !text.contains(TOKEN)), "{exchange:?}");
    }

    #[rstest]
    #[case::with_an_argument(
        r#"{"argv":["--bridge-routes","status"]}"#,
        "`--bridge-routes` is not allowed through the bridge"
    )]
    #[case::another_flag(r#"{"argv":["--bridge-route"]}"#, "`--bridge-route` is not allowed through the bridge")]
    #[tokio::test]
    async fn refuses_any_other_flag_as_before(#[case] argv: &str, #[case] reason: &str) {
        let dir = TempDir::new();
        let served = Served::start(listening(&dir)).await;
        assert_eq!(served.exchange(vec![Message::text(argv)]).await, Exchange::of(&[&stderr(reason), USAGE]));
        assert!(!ran(dir.path()));
    }

    #[rstest]
    #[case::no_token("/")]
    #[case::a_wrong_token("/?token=wrong")]
    #[tokio::test]
    async fn needs_the_token_like_any_command(#[case] target: &str) {
        let dir = TempDir::new();
        let served = Served::start(listening(&dir)).await;
        assert_eq!(served.upgrade(target).await, StatusCode::UNAUTHORIZED);
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
        let mut socket = started(&served).await;
        send(&mut socket, vec![message]).await;
        assert_eq!(Exchange::read(socket).await, Exchange::of(&[&stderr(reason), USAGE]));
        assert!(reaped(number_in(dir.path(), "pid").expect("the fake wrote its pid")));
    }

    #[tokio::test]
    async fn ends_the_command_on_a_message_past_one_mib() {
        let dir = TempDir::new();
        let served = Served::start(faking(&dir, HUNG)).await;
        let mut socket = started(&served).await;
        let MaybeTlsStream::Plain(tcp) = socket.get_mut() else { panic!("a plain connection") };
        let header = [&[0x81, 0xFF][..], &((1_u64 << 20) + 1).to_be_bytes(), &[0; 4]].concat();
        tcp.write_all(&header).await.expect("the header of a text frame past 1 MiB is sent");
        let texts = socket.filter_map(async |message| match message {
            Ok(Message::Text(text)) => Some(text.as_str().to_owned()),
            _ => None,
        });
        assert_eq!(in_time(texts.collect::<Vec<_>>()).await, [stderr("a message passed 1 MiB").as_str(), USAGE]);
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
        const SILENCE: u64 = 1000;
        let dir = TempDir::new();
        let fake = faking(&dir, HUNG);
        let served = Served::start(|config| {
            fake(config);
            heartbeat(20, SILENCE)(config);
        })
        .await;
        let mut socket = started(&served).await;
        let past = Instant::now() + Duration::from_millis(SILENCE * 3 / 2);
        while Instant::now() < past {
            assert!(matches!(in_time(socket.next()).await, Some(Ok(Message::Ping(_)))));
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

mod stopping {
    use super::*;

    const FAMILY: &str = "echo $$ > \"${0%/*}/pid\"\nsleep 30 &\necho $! > \"${0%/*}/grandchild\"\necho ready\nwait";
    const LONG: Duration = Duration::from_secs(60);

    #[tokio::test]
    async fn kills_every_command_and_stops_accepting() {
        let dir = TempDir::new();
        let fake = faking(&dir, FAMILY);
        let served = Served::start(|config| {
            fake(config);
            config.hello = LONG;
            config.handshake = LONG;
        })
        .await;
        let shaking = served.silent().await;
        let mut running = served.connect().await;
        send(&mut running, vec![Message::text(ARGV)]).await;
        assert_eq!(next_text(&mut running).await, READY);
        let greeting = tokio::spawn(Exchange::read(served.connect().await));
        let running = tokio::spawn(Exchange::read(running));
        let pid = number_in(dir.path(), "pid").expect("the fake wrote its pid");
        let grandchild = number_in(dir.path(), "grandchild").expect("the fake wrote its child's pid");
        let url = served.stop().await;
        assert!(reaped(pid));
        wait_until(async || reaped(grandchild)).await;
        assert!(in_time(connect_async(url)).await.is_err());
        assert_eq!(running.await.expect("the client reads"), Exchange::away());
        assert_eq!(greeting.await.expect("the client reads"), Exchange::away());
        closed(shaking).await;
    }
}
