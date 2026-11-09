use rstest::rstest;
use serde_json::json;

use super::*;
use crate::error::Error;
use crate::transport::fake::FakeTransport;
use crate::transport::json;

const TURN_OVER: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/last-message/0.13.19/turn-over.json"));
const MID_TURN: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/last-message/0.13.19/mid-turn.json"));

fn parsed(text: &str) -> LastMessage {
    serde_json::from_str(text).expect("a last message")
}

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

mod reading {
    use super::*;

    #[test]
    fn reads_a_finished_turn_from_cornercase_0_13_19() {
        let message = parsed(TURN_OVER);
        assert_eq!((message.pane, message.agent.as_str()), (543, "claude"));
        assert!(message.text.starts_with("**Resultado: VERDE.**"), "the text: {}", message.text);
        assert!(message.text.contains("\n\n- **Mutation:**"), "the text keeps its lines: {}", message.text);
        assert_eq!(message.written.as_deref(), Some("2026-11-09T14:26:47.027Z"));
        assert_eq!(message.turn_over, Some(true));
    }

    #[test]
    fn reads_a_turn_still_going_from_cornercase_0_13_19() {
        let expected = LastMessage {
            pane: 567,
            agent: "claude".to_owned(),
            text: "Leyendo los patrones del núcleo (cli/status, transport, board) antes de escribir `last_message.rs`."
                .to_owned(),
            written: Some("2026-11-09T14:41:15.074Z".to_owned()),
            turn_over: Some(false),
        };
        assert_eq!(parsed(MID_TURN), expected);
    }

    #[test]
    fn ignores_keys_it_does_not_know() {
        let line = json!({ "pane": 4, "agent": "codex", "text": "hi", "model": "gpt", "header": { "tab": 2 } });
        let message = LastMessage::deserialize(line).expect("a last message");
        let expected =
            LastMessage { pane: 4, agent: "codex".to_owned(), text: "hi".to_owned(), ..LastMessage::default() };
        assert_eq!(message, expected);
    }

    #[test]
    fn leaves_the_time_and_the_turn_empty_when_missing() {
        let message = parsed(r#"{"pane": 4, "agent": "gemini", "text": "done"}"#);
        assert_eq!((message.written, message.turn_over), (None, None));
    }

    #[test]
    fn takes_a_null_time_and_turn() {
        let message = parsed(r#"{"pane": 4, "written": null, "turn_over": null}"#);
        assert_eq!(message, LastMessage { pane: 4, ..LastMessage::default() });
    }
}

mod argv {
    use super::*;

    #[test]
    fn reads_one_pane_as_json() {
        assert_eq!(args(12), words(&["read", "--last-message", "--pane", "12", "--json"]));
    }

    #[test]
    fn sends_from_stdin_with_enter_and_a_timeout() {
        assert_eq!(send_args(12), words(&["send", "--pane", "12", "--enter", "--timeout", "30", "-"]));
    }

    #[test]
    fn forces_a_send_past_the_hold_with_enter_and_a_timeout() {
        let forced = words(&["send", "--pane", "12", "--force", "--enter", "--timeout", "30", "-"]);
        assert_eq!(forced_send_args(12), forced);
    }
}

mod over_a_transport {
    use super::*;

    const READ: &[&str] = &["read", "--last-message", "--pane", "543", "--json"];

    #[tokio::test]
    async fn parses_what_cornercase_prints() {
        let lines: Vec<&str> = TURN_OVER.lines().collect();
        let fake = FakeTransport::default().answer(READ, &lines, &[], 0);
        let message: LastMessage = json(&fake, args(543)).await.expect("a last message");
        assert_eq!(message, parsed(TURN_OVER));
    }

    #[rstest]
    #[case::no_pane("Error: there is no pane 543; `cornercase status` lists them")]
    #[case::no_agent(
        "Error: pane 543 runs no agent, and cornercase only reads the messages of Claude Code, Codex, Gemini CLI, \
         opencode, Grok Build, Cursor CLI, Vibe CLI and GitHub Copilot CLI; `cornercase read --pane 543` prints its screen"
    )]
    #[tokio::test]
    async fn fails_with_the_reason_of_its_one_target(#[case] stderr: &str) {
        let fake = FakeTransport::default().answer(READ, &[], &[stderr], 1);
        let read: Result<LastMessage, Error> = json(&fake, args(543)).await;
        assert_eq!(read, Err(Error::Failed { code: 1, stderr: stderr.to_owned() }));
    }
}
