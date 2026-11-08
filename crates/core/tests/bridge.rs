use std::time::Duration;

use futures_util::{StreamExt, sink, stream};
use rstest::rstest;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use cornercase_bridge::test_util::{
    HUNG, Served, TOKEN, TempDir, fake_cornercase, in_time, number_in, reaped, wait_until,
};
use cornercase_core::error::Error;
use cornercase_core::protocol::{LARGEST_MESSAGE, LONGEST_LINE};
use cornercase_core::transport::bridge::{BridgeConfig, BridgeTransport};
use cornercase_core::transport::{Closed, Exit, Out, OutSink, Stdin, Transport};

const DEADLINE: Duration = Duration::from_millis(200);

struct Bridge {
    served: Served,
    dir: TempDir,
}

impl Bridge {
    async fn start(script: &str) -> Self {
        let dir = TempDir::new();
        let binary = fake_cornercase(dir.path(), script);
        let served = Served::start(move |config| config.binary = binary).await;
        Self { served, dir }
    }

    fn transport(&self, token: &str) -> BridgeTransport {
        BridgeTransport::new(self.served.url(&format!("/?token={token}")), BridgeConfig::default())
    }

    async fn started(&self) -> (JoinHandle<Result<Exit, Error>>, i32) {
        let transport = self.transport(TOKEN);
        let (out, mut heard) = listening();
        let running = tokio::spawn(async move { transport.run(argv(), stream::empty().boxed(), out).await });
        assert_eq!(in_time(heard.recv()).await, Some(Out::Stdout("ready".to_owned())));
        (running, number_in(self.dir.path(), "pid").expect("the fake wrote its pid"))
    }
}

fn argv() -> Vec<String> {
    vec!["status".to_owned()]
}

fn listening() -> (OutSink, UnboundedReceiver<Out>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let out = sink::unfold(sender, async |sender, line| sender.send(line).map(|()| sender).map_err(|_| Closed));
    (Box::pin(out), receiver)
}

async fn run(transport: &dyn Transport, stdin: Vec<Stdin>) -> (Result<Exit, Error>, Vec<Out>) {
    let (out, mut heard) = listening();
    let exit = in_time(transport.run(argv(), stream::iter(stdin).boxed(), out)).await;
    let mut lines = Vec::new();
    while let Some(line) = heard.recv().await {
        lines.push(line);
    }
    (exit, lines)
}

fn exit(code: i32, stderr: &str) -> Exit {
    Exit { code, stderr: stderr.to_owned() }
}

mod run {
    use super::*;

    #[rstest]
    #[case::echoes_the_stdin(
        "exec cat",
        vec![Stdin::Text("hi".to_owned()), Stdin::Text("there".to_owned())],
        vec![Out::Stdout("hi".to_owned()), Out::Stdout("there".to_owned())],
        exit(0, "")
    )]
    #[case::passes_the_stderr_on(
        "echo one >&2\necho two >&2",
        Vec::new(),
        vec![Out::Stderr("one".to_owned()), Out::Stderr("two".to_owned())],
        exit(0, "one\ntwo")
    )]
    #[case::ends_with_the_code("exit 3", Vec::new(), Vec::new(), exit(3, ""))]
    #[tokio::test]
    async fn sends_the_output_and_ends_with_the_exit(
        #[case] script: &str,
        #[case] stdin: Vec<Stdin>,
        #[case] lines: Vec<Out>,
        #[case] ended: Exit,
    ) {
        let bridge = Bridge::start(script).await;
        assert_eq!(run(&bridge.transport(TOKEN), stdin).await, (Ok(ended), lines));
    }

    #[tokio::test]
    async fn delivers_bytes_past_one_mib_whole() {
        let length = 2 * LARGEST_MESSAGE + 1;
        let bridge = Bridge::start("exec wc -c").await;
        let bytes = (0..=u8::MAX).cycle().take(length).collect();
        let (ended, lines) = run(&bridge.transport(TOKEN), vec![Stdin::Bytes(bytes)]).await;
        assert_eq!(ended, Ok(exit(0, "")));
        let [Out::Stdout(count)] = lines.as_slice() else { panic!("one line on stdout, not {lines:?}") };
        assert_eq!(count.trim(), length.to_string());
    }

    #[tokio::test]
    async fn delivers_a_line_of_the_longest_size_whole() {
        let length = LONGEST_LINE - 1;
        let bridge = Bridge::start(&format!("head -c {length} /dev/zero | tr '\\0' a; echo")).await;
        let (ended, lines) = run(&bridge.transport(TOKEN), Vec::new()).await;
        assert_eq!(ended, Ok(exit(0, "")));
        let [Out::Stdout(line)] = lines.as_slice() else { panic!("one line on stdout, not {} lines", lines.len()) };
        assert_eq!(line.len(), length);
    }

    #[tokio::test]
    async fn is_refused_with_a_wrong_token() {
        let bridge = Bridge::start("exit 0").await;
        assert_eq!(run(&bridge.transport("wrong"), Vec::new()).await, (Err(Error::Unauthorized), Vec::new()));
    }

    #[tokio::test]
    async fn drops_when_the_bridge_stops_in_the_middle() {
        let bridge = Bridge::start(HUNG).await;
        let (running, _) = bridge.started().await;
        bridge.served.stop().await;
        assert_eq!(in_time(running).await.expect("the run ends"), Err(Error::Dropped));
    }

    #[tokio::test]
    async fn gives_up_on_a_listener_that_never_answers() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a listener");
        let url = format!("ws://{}/?token={TOKEN}", listener.local_addr().expect("its address"));
        let transport = BridgeTransport::new(url, BridgeConfig { connect_timeout: DEADLINE });
        let started = Instant::now();
        assert_eq!(run(&transport, Vec::new()).await, (Err(Error::Timeout), Vec::new()));
        let waited = started.elapsed();
        assert!(waited >= DEADLINE && waited < 10 * DEADLINE, "it waited {waited:?}");
    }

    #[tokio::test]
    async fn ends_the_command_when_its_future_is_dropped() {
        let bridge = Bridge::start(HUNG).await;
        let (running, pid) = bridge.started().await;
        running.abort();
        assert!(in_time(running).await.expect_err("the run is dropped").is_cancelled());
        wait_until(async || reaped(pid)).await;
    }
}
