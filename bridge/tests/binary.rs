use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use futures_util::{SinkExt, StreamExt};
use rstest::rstest;
use rustix::process::{Pid, Signal, kill_process};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::{Error, Message};

use cornercase_bridge::qr::{self, Place};
use cornercase_bridge::test_util::{HUNG, TempDir, fake_cornercase, in_time, number_in, reaped};

const BRIDGE: &str = env!("CARGO_BIN_EXE_cornercase-bridge");
const LOOPBACK: [&str; 3] = ["--loopback", "--port", "0"];
const QUERY: &str = "/?token=";

fn command(home: &Path) -> Command {
    let mut command = Command::new(BRIDGE);
    command.env("HOME", home).env_remove("XDG_CONFIG_HOME").env_remove("CORNERCASE_BIN");
    command
}

fn token_file(home: &Path) -> PathBuf {
    home.join(".config/cornercase-bridge/token")
}

fn failure(command: &mut Command) -> (Option<i32>, String) {
    let output = command.output().expect("the bridge runs");
    (output.status.code(), String::from_utf8_lossy(&output.stderr).into_owned())
}

struct Bridge {
    child: Child,
    stdout: BufReader<ChildStdout>,
    stderr: ChildStderr,
    printed: String,
}

struct Stopped {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Bridge {
    async fn start(home: &Path, binary: &Path, args: &[&str]) -> Self {
        let mut child = tokio::process::Command::from(command(home))
            .args(LOOPBACK)
            .arg("--cornercase")
            .arg(binary)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("the bridge starts");
        let stdout = BufReader::new(child.stdout.take().expect("the bridge's stdout"));
        let stderr = child.stderr.take().expect("the bridge's stderr");
        let mut bridge = Self { child, stdout, stderr, printed: String::new() };
        bridge.read_the_url().await;
        bridge
    }

    async fn read_the_url(&mut self) {
        while !self.printed.lines().any(|line| line.starts_with("ws://")) {
            let read = in_time(self.stdout.read_line(&mut self.printed)).await.expect("the bridge's stdout");
            assert_ne!(read, 0, "the bridge never printed its URL: {}", self.printed);
        }
    }

    fn url(&self) -> &str {
        self.printed.lines().find(|line| line.starts_with("ws://")).expect("the URL")
    }

    fn token(&self) -> String {
        self.url().split_once(QUERY).expect("the URL has a token").1.to_owned()
    }

    fn with_token(&self, token: &str) -> String {
        format!("{}{QUERY}{token}", self.url().split_once(QUERY).expect("the URL has a token").0)
    }

    async fn stop(mut self, signal: Signal) -> Stopped {
        let pid = self.child.id().and_then(|id| id.try_into().ok()).and_then(Pid::from_raw).expect("the bridge's pid");
        kill_process(pid, signal).expect("the signal is sent");
        let status = in_time(self.child.wait()).await.expect("the bridge exits");
        let mut stderr = String::new();
        in_time(self.stdout.read_to_string(&mut self.printed)).await.expect("the bridge's stdout");
        in_time(self.stderr.read_to_string(&mut stderr)).await.expect("the bridge's stderr");
        Stopped { code: status.code(), stdout: self.printed, stderr }
    }
}

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
        let home = TempDir::new();
        let said = failure(command(home.path()).args(LOOPBACK).args(args).env("PATH", path));
        assert_eq!(said, (Some(2), stderr.to_owned()));
        assert!(!token_file(home.path()).exists(), "no token before the binary is found");
    }

    #[rstest]
    #[case::listen(&["--listen", "127.0.0.1:0"], "unexpected argument '--listen'")]
    #[case::token(&["--token", "t"], "unexpected argument '--token'")]
    #[case::a_port_that_is_not_a_number(&["--port", "x"], "invalid value 'x' for '--port <PORT>'")]
    #[case::a_port_too_high(&["--port", "65536"], "invalid value '65536' for '--port <PORT>'")]
    fn refuses_a_usage_error(#[case] args: &[&str], #[case] reason: &str) {
        let home = TempDir::new();
        let (code, stderr) = failure(command(home.path()).args(args).args(["--cornercase", "/nonexistent"]));
        assert_eq!(code, Some(2), "{stderr}");
        assert!(stderr.contains(reason), "{stderr}");
    }

    #[rstest]
    #[case::unset(None)]
    #[case::relative(Some("home"))]
    fn needs_a_home_for_the_token(#[case] home: Option<&str>) {
        let dir = TempDir::new();
        let mut command = command(dir.path());
        command.args(LOOPBACK);
        match home {
            Some(home) => command.env("HOME", home),
            None => command.env_remove("HOME"),
        };
        let said = failure(command.arg("--cornercase").arg(fake_cornercase(dir.path(), HUNG)));
        let wanted = "cornercase-bridge: HOME is not set to an absolute path, so there is no place for the token\n";
        assert_eq!(said, (Some(2), wanted.to_owned()));
    }

    #[test]
    fn refuses_a_token_file_that_holds_no_token() {
        let home = TempDir::new();
        let path = token_file(home.path());
        fs::create_dir_all(path.parent().expect("a folder")).expect("the folder is made");
        let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path).expect("the file");
        file.write_all(b"not a token!\n").expect("the file is written");
        let said =
            failure(command(home.path()).args(LOOPBACK).arg("--cornercase").arg(fake_cornercase(home.path(), HUNG)));
        let wanted = format!(
            "cornercase-bridge: {} does not hold a token; run with --new-token to make a new one\n",
            path.display()
        );
        assert_eq!(said, (Some(1), wanted));
    }
}

mod printing {
    use super::*;

    #[tokio::test]
    async fn puts_the_url_and_its_code_on_stdout_and_never_the_token_on_stderr() {
        let home = TempDir::new();
        let bridge = Bridge::start(home.path(), &fake_cornercase(home.path(), HUNG), &[]).await;
        let (url, token) = (bridge.url().to_owned(), bridge.token());
        let stopped = bridge.stop(Signal::TERM).await;
        assert_eq!(stopped.stdout, format!("{}\n", qr::block(Place::Loopback, &url).join("\n")));
        assert!(url.starts_with("ws://127.0.0.1:"), "{url}");
        assert_eq!(fs::read_to_string(token_file(home.path())).expect("the token file"), format!("{token}\n"));
        assert!(stopped.stderr.contains("cornercase-bridge: listening on 1 address\n"), "{}", stopped.stderr);
        assert!(!stopped.stderr.contains(&token) && !stopped.stderr.contains("?token"), "{}", stopped.stderr);
    }

    #[tokio::test]
    async fn keeps_the_token_from_one_start_to_the_next() {
        let home = TempDir::new();
        let binary = fake_cornercase(home.path(), HUNG);
        let first = Bridge::start(home.path(), &binary, &[]).await;
        let token = first.token();
        first.stop(Signal::TERM).await;
        let second = Bridge::start(home.path(), &binary, &[]).await;
        assert_eq!(second.token(), token);
    }

    #[tokio::test]
    async fn makes_a_new_token_that_the_old_one_no_longer_opens() {
        let home = TempDir::new();
        let binary = fake_cornercase(home.path(), HUNG);
        let first = Bridge::start(home.path(), &binary, &[]).await;
        let old = first.token();
        first.stop(Signal::TERM).await;
        let renewed = Bridge::start(home.path(), &binary, &["--new-token"]).await;
        assert_ne!(renewed.token(), old);
        let Err(Error::Http(response)) = in_time(connect_async(renewed.with_token(&old))).await else {
            panic!("the old token is refused")
        };
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        in_time(connect_async(renewed.url())).await.expect("the new token opens");
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
        let bridge = Bridge::start(dir.path(), &fake_cornercase(dir.path(), HUNG), &[]).await;
        let (mut socket, _) = in_time(connect_async(bridge.url())).await.expect("the upgrade");
        socket.send(Message::text(r#"{"argv":["status"]}"#)).await.expect("the argv is sent");
        let ready = in_time(socket.next()).await.expect("a message").expect("a message");
        assert_eq!(ready, Message::text(r#"{"stdout":"ready"}"#));
        let pid = number_in(dir.path(), "pid").expect("the fake wrote its pid");
        let token = bridge.token();
        let reading =
            tokio::spawn(async move { socket.filter_map(async |message| message.ok()).collect::<Vec<_>>().await });
        let stopped = bridge.stop(signal).await;
        let farewell = in_time(reading).await.expect("the client reads to the end");
        assert!(matches!(farewell.as_slice(), [.., Message::Close(Some(frame))] if frame.code == CloseCode::Away));
        assert_eq!((stopped.code, reaped(pid)), (Some(0), true), "{}", stopped.stderr);
        assert!(!stopped.stderr.contains(&token), "{}", stopped.stderr);
    }
}
