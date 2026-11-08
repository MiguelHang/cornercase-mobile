use std::io::ErrorKind;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::ExitStatus;

use rstest::rstest;
use rustix::io::Errno;
use rustix::process::{Pid, test_kill_process};

use super::*;
use crate::test_util::{TempDir, collect, in_time, wait_until, write_executable};

fn fake(dir: &TempDir, script: &str) -> PathBuf {
    let path = dir.path().join("cornercase");
    write_executable(&path, &format!("#!/bin/sh\n{script}\n"));
    path
}

fn start(dir: &TempDir, script: &str, argv: &[&str]) -> Running {
    let argv: Vec<String> = argv.iter().map(|argument| (*argument).to_owned()).collect();
    spawn(&fake(dir, script), &argv).expect("the fake starts")
}

async fn run(script: &str, argv: &[&str]) -> Vec<BridgeMessage> {
    let dir = TempDir::new();
    collect(&mut start(&dir, script, argv)).await
}

fn stdout(line: &str) -> BridgeMessage {
    BridgeMessage::Stdout(line.to_owned())
}

mod spawn {
    use super::*;

    #[rstest]
    #[case::an_exit_code("echo a; exit 3", &[], vec![stdout("a"), BridgeMessage::Exit(3)])]
    #[case::a_signal("kill -TERM $$", &[], vec![BridgeMessage::Exit(143)])]
    #[case::arguments_without_expansion(
        "printf '%s\\n' \"$@\"",
        &["*", "$HOME", "a b"],
        vec![stdout("*"), stdout("$HOME"), stdout("a b"), BridgeMessage::Exit(0)]
    )]
    #[tokio::test]
    async fn ends_with_one_exit_after_the_output(
        #[case] script: &str,
        #[case] argv: &[&str],
        #[case] expected: Vec<BridgeMessage>,
    ) {
        assert_eq!(run(script, argv).await, expected);
    }

    #[tokio::test]
    async fn keeps_each_stream_in_order() {
        let messages = run("echo a; echo x >&2; echo b; echo y >&2; echo c", &[]).await;
        let (errors, rest): (Vec<_>, Vec<_>) =
            messages.into_iter().partition(|message| matches!(message, BridgeMessage::Stderr(_)));
        assert_eq!(
            (errors, rest),
            (
                vec![BridgeMessage::Stderr("x".to_owned()), BridgeMessage::Stderr("y".to_owned())],
                vec![stdout("a"), stdout("b"), stdout("c"), BridgeMessage::Exit(0)]
            )
        );
    }

    #[rstest]
    #[case::not_executable(false, ErrorKind::PermissionDenied)]
    #[case::deleted(true, ErrorKind::NotFound)]
    #[tokio::test]
    async fn fails_when_the_binary_cannot_run(#[case] deleted: bool, #[case] kind: ErrorKind) {
        let dir = TempDir::new();
        let path = dir.path().join("cornercase");
        if deleted {
            write_executable(&path, "#!/bin/sh\n");
            std::fs::remove_file(&path).expect("delete the file");
        } else {
            std::fs::write(&path, "#!/bin/sh\n").expect("write the file");
        }
        assert_eq!(spawn(&path, &[]).err().map(|error| error.kind()), Some(kind));
    }
}

mod write {
    use super::*;

    fn text(line: &str) -> Input {
        Input::Text(line.to_owned())
    }

    #[rstest]
    #[case::lines_of_text(vec![text("a"), text("b")], vec![stdout("a"), stdout("b")])]
    #[case::bytes_as_they_are(vec![Input::Bytes(b"ab".to_vec()), text("c")], vec![stdout("abc")])]
    #[tokio::test]
    async fn reaches_the_child_until_the_end_of_its_input(
        #[case] inputs: Vec<Input>,
        #[case] lines: Vec<BridgeMessage>,
    ) {
        let dir = TempDir::new();
        let mut running = start(&dir, "cat", &[]);
        for input in inputs.into_iter().chain([Input::Eof]) {
            running.write(input).await.expect("the input is accepted");
        }
        assert_eq!(collect(&mut running).await, [lines, vec![BridgeMessage::Exit(0)]].concat());
    }

    #[tokio::test]
    async fn fails_after_the_end_of_the_input() {
        let dir = TempDir::new();
        let mut running = start(&dir, "cat", &[]);
        running.write(Input::Eof).await.expect("the end is accepted");
        let refused = running.write(text("a")).await.err().map(|error| error.kind());
        assert_eq!((refused, collect(&mut running).await), (Some(ErrorKind::BrokenPipe), vec![BridgeMessage::Exit(0)]));
    }

    #[tokio::test]
    async fn fails_once_the_child_stops_reading() {
        let dir = TempDir::new();
        let mut running = start(&dir, "exit 4", &[]);
        assert_eq!(collect(&mut running).await, [BridgeMessage::Exit(4)]);
        wait_until(async || running.write(text("a")).await.is_err()).await;
        assert_eq!(running.next().await, None);
    }
}

mod kill {
    use super::*;

    const HUNG: &str = "sleep 30 & echo $$ $!; wait";
    const STUBBORN: &str = "trap '' TERM; sleep 30 & echo $$ $!; wait";

    async fn pids(running: &mut Running) -> (i32, i32) {
        let line = match in_time(running.next()).await {
            Some(BridgeMessage::Stdout(line)) => line,
            other => panic!("the fake prints its pids, not {other:?}"),
        };
        let (leader, sleep) = line.split_once(' ').expect("two pids");
        (leader.parse().expect("the leader's pid"), sleep.parse().expect("the sleep's pid"))
    }

    fn reaped(pid: i32) -> bool {
        Pid::from_raw(pid).is_some_and(|pid| matches!(test_kill_process(pid), Err(Errno::SRCH)))
    }

    fn gone(pid: i32) -> bool {
        reaped(pid) || zombie(pid)
    }

    fn zombie(pid: i32) -> bool {
        std::process::Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).trim_start().starts_with('Z'))
    }

    #[tokio::test]
    async fn dropping_it_ends_the_whole_group() {
        let dir = TempDir::new();
        let mut running = start(&dir, HUNG, &[]);
        let (leader, sleep) = pids(&mut running).await;
        drop(running);
        wait_until(async || reaped(leader) && gone(sleep)).await;
    }

    #[rstest]
    #[case::by_terminating_it(HUNG, 143)]
    #[case::by_killing_what_ignores_termination(STUBBORN, 137)]
    #[tokio::test]
    async fn ends_the_whole_group_and_returns_once_reaped(#[case] script: &str, #[case] code: i32) {
        let dir = TempDir::new();
        let mut running = start(&dir, script, &[]);
        let (leader, sleep) = pids(&mut running).await;
        assert_eq!((in_time(running.kill()).await, reaped(leader)), (code, true));
        wait_until(async || gone(sleep)).await;
    }

    #[tokio::test]
    async fn returns_the_real_code_once_the_child_has_exited() {
        let dir = TempDir::new();
        let mut running = start(&dir, "exit 5", &[]);
        assert_eq!(collect(&mut running).await, [BridgeMessage::Exit(5)]);
        assert_eq!(running.kill().await, 5);
    }
}

mod exit_code {
    use super::*;

    #[rstest]
    #[case::a_code(Ok(ExitStatus::from_raw(3 << 8)), 3)]
    #[case::a_signal(Ok(ExitStatus::from_raw(9)), 137)]
    #[case::a_failed_wait(Err(std::io::Error::from(ErrorKind::Other)), -1)]
    fn follows_the_shell_convention(#[case] status: std::io::Result<ExitStatus>, #[case] code: i32) {
        assert_eq!(group::exit_code(status), code);
    }
}
