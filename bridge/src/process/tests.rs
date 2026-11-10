use std::io::ErrorKind;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use std::time::Duration;

use rstest::rstest;
use tokio::time::timeout;

use cornercase_core::protocol::{Input, LINE_TOO_LONG, USAGE_EXIT};

use super::*;
use crate::test_util::{TempDir, collect, fake_cornercase, in_time, reaped, wait_until, write_executable};

fn start(dir: &TempDir, script: &str, argv: &[&str]) -> Running {
    let argv: Vec<String> = argv.iter().map(|argument| (*argument).to_owned()).collect();
    spawn(&fake_cornercase(dir.path(), script), &argv).expect("the fake starts")
}

async fn run(script: &str, argv: &[&str]) -> Vec<BridgeMessage> {
    let dir = TempDir::new();
    collect(&mut start(&dir, script, argv)).await
}

fn stdout(line: &str) -> BridgeMessage {
    BridgeMessage::Stdout(line.to_owned())
}

async fn pids(running: &mut Running) -> (i32, i32) {
    let line = match in_time(running.next()).await {
        Some(BridgeMessage::Stdout(line)) => line,
        other => panic!("the fake prints two pids, not {other:?}"),
    };
    let (first, second) = line.split_once(' ').expect("two pids");
    (first.parse().expect("the first pid"), second.parse().expect("the second pid"))
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

    #[tokio::test]
    async fn delivers_a_line_of_more_than_a_mebibyte_in_one_message() {
        let messages = run("head -c 1048577 /dev/zero | tr '\\0' a; echo", &[]).await;
        assert_eq!(messages, [stdout(&"a".repeat((1 << 20) + 1)), BridgeMessage::Exit(0)]);
    }

    #[rstest]
    #[case::by_the_command("echo $$ $$; exec tr '\\0' a </dev/zero")]
    #[case::by_a_grandchild("(sleep 0.2; exec tr '\\0' a </dev/zero) & echo $$ $!")]
    #[tokio::test]
    async fn ends_with_a_usage_error_once_a_line_passes_the_limit(#[case] script: &str) {
        let dir = TempDir::new();
        let mut running = launch(&fake_cornercase(dir.path(), script), &[], 4096).expect("the fake starts");
        let (_, writer) = pids(&mut running).await;
        let rest = collect(&mut running).await;
        assert_eq!(rest, [BridgeMessage::Stderr(LINE_TOO_LONG.to_owned()), BridgeMessage::Exit(USAGE_EXIT)]);
        wait_until(async || gone(writer)).await;
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

    #[tokio::test]
    async fn waits_for_the_output_a_grandchild_still_writes_before_the_exit() {
        let messages = run("(sleep 0.2; echo late) &", &[]).await;
        assert_eq!(messages, [stdout("late"), BridgeMessage::Exit(0)]);
    }

    #[tokio::test]
    async fn sends_a_last_line_without_a_newline_while_a_grandchild_holds_the_pipe() {
        assert_eq!(run("sleep 30 & printf partial", &[]).await, [stdout("partial"), BridgeMessage::Exit(0)]);
    }

    #[tokio::test]
    async fn keeps_the_output_nobody_read_before_the_patience_ran_out() {
        let dir = TempDir::new();
        let mut running = start(&dir, "echo $$ $$; seq 100", &[]);
        let (leader, _) = pids(&mut running).await;
        wait_until(async || reaped(leader)).await;
        tokio::time::sleep(group::PATIENCE + Duration::from_millis(500)).await;
        let lines = (1..=100).map(|n| stdout(&n.to_string()));
        assert_eq!(collect(&mut running).await, lines.chain([BridgeMessage::Exit(0)]).collect::<Vec<_>>());
    }

    #[rstest]
    #[case::holding_its_output("sleep 30 & echo $$ $!")]
    #[case::without_its_output("sleep 30 >/dev/null 2>&1 & echo $$ $!")]
    #[tokio::test]
    async fn ends_a_grandchild_left_in_the_group_before_the_exit(#[case] script: &str) {
        let dir = TempDir::new();
        let mut running = start(&dir, script, &[]);
        let (_, sleep) = pids(&mut running).await;
        let rest = timeout(group::PATIENCE * 2, collect(&mut running)).await;
        assert_eq!(rest.ok(), Some(vec![BridgeMessage::Exit(0)]));
        wait_until(async || gone(sleep)).await;
    }
}

mod write {
    use super::*;

    fn text(line: &str) -> Input {
        Input::Text(line.to_owned())
    }

    async fn feed(stdin: &mut Stdin, lines: &[String]) {
        for line in lines {
            stdin.write(text(line)).await.expect("the input is accepted");
        }
        stdin.write(Input::Eof).await.expect("the end is accepted");
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
        let mut stdin = running.take_stdin().expect("its input");
        for input in inputs.into_iter().chain([Input::Eof]) {
            stdin.write(input).await.expect("the input is accepted");
        }
        assert_eq!(collect(&mut running).await, [lines, vec![BridgeMessage::Exit(0)]].concat());
    }

    #[tokio::test]
    async fn fails_after_the_end_of_the_input() {
        let dir = TempDir::new();
        let mut running = start(&dir, "cat", &[]);
        let mut stdin = running.take_stdin().expect("its input");
        stdin.write(Input::Eof).await.expect("the end is accepted");
        let refused = stdin.write(text("a")).await.err().map(|error| error.kind());
        assert_eq!((refused, collect(&mut running).await), (Some(ErrorKind::BrokenPipe), vec![BridgeMessage::Exit(0)]));
    }

    #[tokio::test]
    async fn fails_once_the_child_stops_reading() {
        let dir = TempDir::new();
        let mut running = start(&dir, "exit 4", &[]);
        let mut stdin = running.take_stdin().expect("its input");
        assert_eq!(collect(&mut running).await, [BridgeMessage::Exit(4)]);
        wait_until(async || stdin.write(text("a")).await.is_err()).await;
        assert_eq!(running.next().await, None);
    }

    #[tokio::test]
    async fn is_written_while_the_output_is_read() {
        let dir = TempDir::new();
        let mut running = start(&dir, "cat", &[]);
        let mut stdin = running.take_stdin().expect("its input");
        let lines: Vec<String> = (0..200).map(|n| n.to_string()).collect();
        let ((), messages) = tokio::join!(feed(&mut stdin, &lines), collect(&mut running));
        let expected = lines.iter().map(|line| stdout(line)).chain([BridgeMessage::Exit(0)]);
        assert_eq!(messages, expected.collect::<Vec<_>>());
    }
}

mod kill {
    use super::*;

    const HUNG: &str = "sleep 30 & echo $$ $!; wait";
    const STUBBORN: &str = "trap '' TERM; sleep 30 & echo $$ $!; wait";
    const LEFT_BEHIND: &str = r#"sh -c 'trap "" TERM; echo $PPID $$; exec sleep 30' & wait"#;

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
    #[case::by_killing_a_member_that_outlives_its_leader(LEFT_BEHIND, 143)]
    #[tokio::test]
    async fn ends_the_whole_group_and_returns_once_reaped(#[case] script: &str, #[case] code: i32) {
        let dir = TempDir::new();
        let mut running = start(&dir, script, &[]);
        let (leader, sleep) = pids(&mut running).await;
        assert_eq!((in_time(running.kill()).await, reaped(leader)), (code, true));
        wait_until(async || gone(sleep)).await;
    }

    #[tokio::test]
    async fn gives_a_slow_terminating_leader_time_before_killing_it() {
        let dir = TempDir::new();
        let mut running = start(&dir, "trap 'sleep 0.5; exit 7' TERM; sleep 30 & echo $$ $!; wait", &[]);
        let (leader, sleep) = pids(&mut running).await;
        assert_eq!((in_time(running.kill()).await, reaped(leader)), (7, true));
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
