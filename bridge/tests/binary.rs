use std::path::Path;
use std::process::{Command, Stdio};

use futures_util::{SinkExt, StreamExt};
use rstest::rstest;
use rustix::io::Errno;
use rustix::process::{Pid, Signal, kill_process, test_kill_process};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, ChildStderr};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use cornercase_bridge::test_util::{TempDir, fake_cornercase, in_time};

const BRIDGE: &str = env!("CARGO_BIN_EXE_cornercase-bridge");
const LOCAL: [&str; 4] = ["--listen", "127.0.0.1:0", "--token", "t"];
const TOKEN: &str = "s3cret-token";
const LISTENING: &str = "cornercase-bridge: listening on 127.0.0.1:";
const HUNG: &str = "echo $$ > \"${0%/*}/pid\"\necho ready\nexec sleep 30";

mod starting {
    use super::*;

    #[rstest]
    #[case::option_is_missing(
        &["--cornercase", "/nonexistent"],
        "/usr/bin:/bin",
        "cornercase-bridge: --cornercase: /nonexistent does not exist or is not executable\n"
    )]
    #[case::nothing_in_an_empty_path(&[], "", "cornercase-bridge: no executable `cornercase` in the PATH ()\n")]
    fn needs_a_cornercase_binary(#[case] args: &[&str], #[case] path: &str, #[case] stderr: &str) {
        let output = Command::new(BRIDGE)
            .args(LOCAL)
            .args(args)
            .env_remove("CORNERCASE_BIN")
            .env("PATH", path)
            .output()
            .expect("the bridge runs");
        assert_eq!((output.status.code(), String::from_utf8_lossy(&output.stderr).as_ref()), (Some(2), stderr));
    }

    #[rstest]
    #[case::no_listen(&["--token", "t"], "--listen <ADDRESS>")]
    #[case::no_token(&["--listen", "127.0.0.1:0"], "--token <TOKEN>")]
    #[case::an_empty_token(&["--listen", "127.0.0.1:0", "--token", ""], "a value is required for '--token <TOKEN>'")]
    #[case::any_ipv4(&["--listen", "0.0.0.0:0", "--token", "t"], "0.0.0.0 is not a loopback address")]
    #[case::any_ipv6(&["--listen", "[::]:0", "--token", "t"], ":: is not a loopback address")]
    #[case::a_lan_address(&["--listen", "192.168.1.10:0", "--token", "t"], "comes with task 006")]
    fn refuses_a_usage_error(#[case] args: &[&str], #[case] reason: &str) {
        let output =
            Command::new(BRIDGE).args(args).args(["--cornercase", "/nonexistent"]).output().expect("the bridge runs");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{stderr}");
        assert!(stderr.contains(reason), "{stderr}");
    }
}

mod stopping {
    use super::*;

    #[rstest]
    #[case::interrupt(Signal::INT)]
    #[case::terminate(Signal::TERM)]
    #[tokio::test]
    async fn kills_the_command_and_exits_cleanly(#[case] signal: Signal) {
        let dir = TempDir::new();
        let mut bridge = start(&fake_cornercase(dir.path(), HUNG));
        let mut stderr = BufReader::new(bridge.stderr.take().expect("the bridge's stderr"));
        let (port, mut said) = listening(&mut stderr).await;
        let url = format!("ws://127.0.0.1:{port}/?token={TOKEN}");
        let (mut socket, _) = in_time(connect_async(url)).await.expect("the upgrade");
        socket.send(Message::text(r#"{"argv":["status"]}"#)).await.expect("the argv is sent");
        let ready = in_time(socket.next()).await.expect("a message").expect("a message");
        assert_eq!(ready, Message::text(r#"{"stdout":"ready"}"#));
        let pid = std::fs::read_to_string(dir.path().join("pid")).expect("the fake wrote its pid");
        let pid = Pid::from_raw(pid.trim().parse().expect("a pid")).expect("a positive pid");
        let bridge_pid = Pid::from_raw(bridge.id().and_then(|id| id.try_into().ok()).expect("the bridge's pid"));
        let reading =
            tokio::spawn(async move { socket.filter_map(async |message| message.ok()).collect::<Vec<_>>().await });
        kill_process(bridge_pid.expect("a positive pid"), signal).expect("the signal is sent");
        let status = in_time(bridge.wait()).await.expect("the bridge exits");
        let farewell = in_time(reading).await.expect("the client reads to the end");
        assert!(matches!(farewell.as_slice(), [.., Message::Close(Some(frame))] if frame.code == CloseCode::Normal));
        in_time(stderr.read_to_string(&mut said)).await.expect("the bridge's stderr");
        assert_eq!((status.code(), test_kill_process(pid)), (Some(0), Err(Errno::SRCH)), "{said}");
        assert!(!said.contains(TOKEN), "{said}");
    }
}

fn start(binary: &Path) -> Child {
    tokio::process::Command::new(BRIDGE)
        .args(["--listen", "127.0.0.1:0", "--token", TOKEN, "--cornercase"])
        .arg(binary)
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("the bridge starts")
}

async fn listening(stderr: &mut BufReader<ChildStderr>) -> (u16, String) {
    let mut said = String::new();
    loop {
        let start = said.len();
        let read = in_time(stderr.read_line(&mut said)).await.expect("the bridge's stderr");
        assert_ne!(read, 0, "the bridge never said where it listens: {said}");
        if let Some(port) = said[start..].trim_end().strip_prefix(LISTENING) {
            return (port.parse().expect("a port"), said);
        }
    }
}
