use std::time::Duration;

use futures_util::{StreamExt, sink, stream};
use rstest::rstest;
use serde_json::json;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use cornercase_core::error::Error;
use cornercase_core::machines::code::MachineCode;
use cornercase_core::machines::pick::{PickConfig, pick_route, websocket_transports};
use cornercase_core::machines::route::{Route, Token};
use cornercase_core::machines::{Machine, MachineId};
use cornercase_core::protocol::{LARGEST_MESSAGE, LONGEST_LINE, ServerMessage};
use cornercase_core::transport::fake::cli::{CLI, Heard, Peer, heard, listening, request, say, serve};
use cornercase_core::transport::websocket::{SocketConfig, WebSocketTransport};
use cornercase_core::transport::{Closed, Exit, Out, OutSink, Stdin, Transport};

const SECRET: &str = "4f2a91c0.9c1e2a7b5d0f";
const DEADLINE: Duration = Duration::from_millis(200);

async fn within<T>(work: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), work).await.expect("it gets there within ten seconds")
}

fn transport(url: String, secret: &str) -> WebSocketTransport {
    WebSocketTransport::new(url, SocketConfig::default()).bearing(secret.to_owned())
}

fn argv() -> Vec<String> {
    vec!["status".to_owned()]
}

fn listening_to() -> (OutSink, UnboundedReceiver<Out>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let out = sink::unfold(sender, async |sender, line| sender.send(line).map(|()| sender).map_err(|_| Closed));
    (Box::pin(out), receiver)
}

async fn run(transport: &dyn Transport, stdin: Vec<Stdin>) -> (Result<Exit, Error>, Vec<Out>) {
    let (out, mut heard) = listening_to();
    let exit = within(transport.run(argv(), stream::iter(stdin).boxed(), out)).await;
    let mut lines = Vec::new();
    while let Some(line) = heard.recv().await {
        lines.push(line);
    }
    (exit, lines)
}

async fn answering(answer: impl FnOnce(&Heard) -> Vec<ServerMessage> + Send + 'static) -> (String, JoinHandle<Heard>) {
    let (url, listener) = listening(CLI).await;
    let script = serve(listener, SECRET, async move |mut peer: Peer| {
        let heard = request(&heard(&mut peer).await);
        say(&mut peer, &answer(&heard)).await;
        heard
    });
    (url, script)
}

async fn started() -> (JoinHandle<()>, JoinHandle<Result<Exit, Error>>) {
    let (url, listener) = listening(CLI).await;
    let served = serve(listener, SECRET, async |mut peer: Peer| {
        peer.next().await;
        say(&mut peer, &[ServerMessage::Stdout("ready".to_owned())]).await;
        while let Some(Ok(_)) = peer.next().await {}
    });
    let transport = transport(url, SECRET);
    let (out, mut heard) = listening_to();
    let running = tokio::spawn(async move { transport.run(argv(), stream::pending().boxed(), out).await });
    assert_eq!(within(heard.recv()).await, Some(Out::Stdout("ready".to_owned())));
    (served, running)
}

fn exit(code: i32, stderr: &str) -> Exit {
    Exit { code, stderr: stderr.to_owned() }
}

fn echo(heard: &Heard) -> Vec<ServerMessage> {
    let lines = heard.stdin.iter().filter_map(|input| match input {
        Stdin::Text(text) => Some(ServerMessage::Stdout(text.clone())),
        Stdin::Bytes(_) => None,
    });
    lines.chain([ServerMessage::Exit(0)]).collect()
}

fn complain(_: &Heard) -> Vec<ServerMessage> {
    let lines = [ServerMessage::Stderr("one".to_owned()), ServerMessage::Stderr("two".to_owned())];
    lines.into_iter().chain([ServerMessage::Exit(0)]).collect()
}

fn fail(_: &Heard) -> Vec<ServerMessage> {
    vec![ServerMessage::Exit(3)]
}

mod run {
    use super::*;

    #[rstest]
    #[case::echoes_the_stdin(
        echo,
        vec![Stdin::Text("hi".to_owned()), Stdin::Text("there".to_owned())],
        vec![Out::Stdout("hi".to_owned()), Out::Stdout("there".to_owned())],
        exit(0, "")
    )]
    #[case::passes_the_stderr_on(
        complain,
        Vec::new(),
        vec![Out::Stderr("one".to_owned()), Out::Stderr("two".to_owned())],
        exit(0, "one\ntwo")
    )]
    #[case::ends_with_the_code(fail, Vec::new(), Vec::new(), exit(3, ""))]
    #[tokio::test]
    async fn sends_the_output_and_ends_with_the_exit(
        #[case] answer: fn(&Heard) -> Vec<ServerMessage>,
        #[case] stdin: Vec<Stdin>,
        #[case] lines: Vec<Out>,
        #[case] ended: Exit,
    ) {
        let (url, script) = answering(answer).await;
        assert_eq!(run(&transport(url, SECRET), stdin).await, (Ok(ended), lines));
        assert_eq!(within(script).await.expect("the script").argv, argv());
    }

    #[tokio::test]
    async fn delivers_bytes_past_one_mib_whole() {
        let bytes: Vec<u8> = (0..=u8::MAX).cycle().take(2 * LARGEST_MESSAGE + 1).collect();
        let (url, script) = answering(|_| vec![ServerMessage::Exit(0)]).await;
        assert_eq!(
            run(&transport(url, SECRET), vec![Stdin::Bytes(bytes.clone())]).await,
            (Ok(exit(0, "")), Vec::new())
        );
        let heard = within(script).await.expect("the script");
        let delivered: Vec<u8> = heard
            .stdin
            .into_iter()
            .flat_map(|input| match input {
                Stdin::Bytes(piece) => piece,
                Stdin::Text(text) => panic!("bytes, not the text {text:?}"),
            })
            .collect();
        assert!(delivered == bytes, "{} bytes came of {}", delivered.len(), bytes.len());
    }

    #[tokio::test]
    async fn delivers_a_line_of_the_longest_size_whole() {
        let length = LONGEST_LINE - 1;
        let (url, _script) =
            answering(move |_| vec![ServerMessage::Stdout("a".repeat(length)), ServerMessage::Exit(0)]).await;
        let (ended, lines) = run(&transport(url, SECRET), Vec::new()).await;
        assert_eq!(ended, Ok(exit(0, "")));
        let [Out::Stdout(line)] = lines.as_slice() else { panic!("one line on stdout, not {} lines", lines.len()) };
        assert_eq!(line.len(), length);
    }

    #[tokio::test]
    async fn is_refused_with_a_secret_it_does_not_know() {
        let (url, _script) = answering(fail).await;
        assert_eq!(run(&transport(url, "wrong"), Vec::new()).await, (Err(Error::NotPaired), Vec::new()));
    }

    #[tokio::test]
    async fn drops_when_the_server_stops_in_the_middle() {
        let (served, running) = started().await;
        served.abort();
        assert_eq!(within(running).await.expect("the run ends"), Err(Error::Dropped));
    }

    #[tokio::test]
    async fn gives_up_on_a_listener_that_never_answers() {
        let (url, _listener) = listening(CLI).await;
        let config = SocketConfig { connect_timeout: DEADLINE, ..SocketConfig::default() };
        let transport = WebSocketTransport::new(url, config).bearing(SECRET.to_owned());
        let started = Instant::now();
        assert_eq!(run(&transport, Vec::new()).await, (Err(Error::Timeout), Vec::new()));
        let waited = started.elapsed();
        assert!(waited >= DEADLINE && waited < 10 * DEADLINE, "it waited {waited:?}");
    }

    #[tokio::test]
    async fn ends_the_command_when_its_future_is_dropped() {
        let (served, running) = started().await;
        running.abort();
        assert!(within(running).await.expect_err("the run is dropped").is_cancelled());
        within(served).await.expect("the server sees the connection end");
    }
}

mod pick_route {
    use super::*;

    fn route(url: &str) -> Route {
        let code = MachineCode::parse(&format!("{}?code=c", url.trim_end_matches("cli"))).expect("a code");
        code.cli_routes(&Token::fake(SECRET)).remove(0)
    }

    #[tokio::test]
    async fn skips_a_closed_port_and_takes_the_contract_of_the_server_that_answers() {
        let (url, listener) = listening(CLI).await;
        drop(listener);
        let closed = route(&url);
        let (url, script) =
            answering(|_| vec![ServerMessage::Stdout(r#"{"version":1}"#.to_owned()), ServerMessage::Exit(0)]).await;
        let live = route(&url);
        let machine = Machine {
            id: MachineId(1),
            name: "studio".to_owned(),
            routes: vec![closed, live.clone()],
            last_route: None,
        };
        let factory = websocket_transports(SocketConfig::default());
        let picked = within(pick_route(&machine, factory, PickConfig::default())).await.expect("the live route wins");
        assert_eq!((picked.route, picked.contract), (live, json!({ "version": 1 })));
        assert_eq!(within(script).await.expect("the script").argv, ["contract", "--json"]);
    }
}
