use futures_util::{StreamExt, sink};
use rstest::rstest;
use tokio::sync::mpsc;

use super::*;
use crate::transport::listen;

fn words(words: &[&str]) -> Vec<String> {
    owned(words)
}

fn no_stdin() -> BoxStream<'static, Stdin> {
    stream::empty().boxed()
}

fn received(mut receiver: mpsc::UnboundedReceiver<Out>) -> Vec<Out> {
    std::iter::from_fn(|| receiver.try_recv().ok()).collect()
}

async fn run(fake: &FakeTransport, argv: &[&str]) -> (Result<Exit, Error>, Vec<Out>) {
    let (out, receiver) = listen();
    let result = fake.run(words(argv), no_stdin(), out).await;
    (result, received(receiver))
}

mod answer {
    use super::*;

    #[tokio::test]
    async fn sends_its_lines_and_exits_with_its_code() {
        let fake = FakeTransport::default().answer(&["status"], &["one", "two"], &["warn", "late"], 3);
        let (result, lines) = run(&fake, &["status"]).await;
        assert_eq!(result, Ok(Exit { code: 3, stderr: "warn\nlate".to_owned() }));
        let expected = [
            Out::Stdout("one".to_owned()),
            Out::Stdout("two".to_owned()),
            Out::Stderr("warn".to_owned()),
            Out::Stderr("late".to_owned()),
        ];
        assert_eq!(lines, expected);
    }

    #[tokio::test]
    async fn answers_every_run() {
        let fake = FakeTransport::default().answer(&["status"], &["one"], &[], 0);
        for _ in 0..2 {
            let (result, lines) = run(&fake, &["status"]).await;
            assert_eq!(result, Ok(Exit { code: 0, stderr: String::new() }));
            assert_eq!(lines, [Out::Stdout("one".to_owned())]);
        }
    }

    #[rstest]
    #[case::shorter(&["status"])]
    #[case::longer(&["status", "--json", "--all"])]
    #[case::reordered(&["--json", "status"])]
    #[tokio::test]
    async fn needs_the_exact_argv(#[case] argv: &[&str]) {
        let fake = FakeTransport::default().answer(&["status", "--json"], &["{}"], &[], 0);
        let (result, lines) = run(&fake, argv).await;
        assert_eq!(result, Err(Error::Unscripted(words(argv))));
        assert_eq!(lines, []);
    }

    #[tokio::test]
    async fn reports_a_closed_output() {
        let fake = FakeTransport::default().answer(&["status"], &["one"], &[], 0);
        let out: OutSink = Box::pin(sink::unfold((), async |(), _: Out| Err::<(), _>(Closed)));
        let result = fake.run(words(&["status"]), no_stdin(), out).await;
        assert_eq!(result, Err(Error::OutputClosed));
    }
}

mod script {
    use super::*;

    fn echo() -> FakeTransport {
        FakeTransport::default().script(&["cat"], |mut stdin, mut out| async move {
            while let Some(input) = stdin.next().await {
                let line = match input {
                    Stdin::Text(text) => text,
                    Stdin::Bytes(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                };
                out.send(Out::Stdout(line)).await.map_err(|Closed| Error::OutputClosed)?;
            }
            Ok(Exit { code: 0, stderr: String::new() })
        })
    }

    #[tokio::test]
    async fn echoes_the_stdin_while_it_comes() {
        let (input, inputs) = mpsc::unbounded_channel();
        let stdin = stream::unfold(inputs, async |mut inputs| inputs.recv().await.map(|line| (line, inputs))).boxed();
        let (out, mut receiver) = listen();
        let fake = echo();
        let running = tokio::spawn(async move { fake.run(words(&["cat"]), stdin, out).await });
        input.send(Stdin::Text("first".to_owned())).expect("the script reads its stdin");
        assert_eq!(receiver.recv().await, Some(Out::Stdout("first".to_owned())));
        input.send(Stdin::Bytes(b"second".to_vec())).expect("the script reads its stdin");
        assert_eq!(receiver.recv().await, Some(Out::Stdout("second".to_owned())));
        drop(input);
        assert_eq!(running.await.expect("the script ends"), Ok(Exit { code: 0, stderr: String::new() }));
        assert_eq!(receiver.recv().await, None);
    }
}

mod fail {
    use super::*;

    #[rstest]
    #[case::unauthorized(Error::Unauthorized)]
    #[case::dropped(Error::Dropped)]
    #[case::timeout(Error::Timeout)]
    #[case::rejected(Error::Rejected { status: 403 })]
    #[tokio::test]
    async fn returns_its_error_every_run(#[case] error: Error) {
        let fake = FakeTransport::default().fail(&["status"], error.clone());
        for _ in 0..2 {
            let (result, lines) = run(&fake, &["status"]).await;
            assert_eq!(result, Err(error.clone()));
            assert_eq!(lines, []);
        }
    }
}

mod unscripted {
    use super::*;

    #[tokio::test]
    async fn names_the_argv_instead_of_panicking() {
        let (result, lines) = run(&FakeTransport::default(), &["projects", "--json"]).await;
        assert_eq!(result, Err(Error::Unscripted(words(&["projects", "--json"]))));
        assert_eq!(result.expect_err("unscripted").to_string(), r#"no answer is scripted for ["projects", "--json"]"#);
        assert_eq!(lines, []);
    }
}
