use std::io::ErrorKind;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::ExitStatus;

use rstest::rstest;

use super::*;
use crate::test_util::{TempDir, collect, write_executable};

fn fake(dir: &TempDir, script: &str) -> PathBuf {
    let path = dir.path().join("cornercase");
    write_executable(&path, &format!("#!/bin/sh\n{script}\n"));
    path
}

async fn run(script: &str, argv: &[&str]) -> Vec<BridgeMessage> {
    let dir = TempDir::new();
    let argv: Vec<String> = argv.iter().map(|argument| (*argument).to_owned()).collect();
    let mut running = spawn(&fake(&dir, script), &argv).expect("the fake starts");
    collect(&mut running).await
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
