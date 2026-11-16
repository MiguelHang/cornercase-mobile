use std::sync::Arc;
use std::time::Duration;

use rstest::rstest;
use tokio::time;

use crate::board::AgentRow;
use crate::cli::last_message::{self, LastMessage};
use crate::demo::DemoTransport;
use crate::demo::tests::{entered, status};
use crate::queue::Outcome;
use crate::store::Store;
use crate::transport::{self, Output, Stdin, Transport};

const RESTING: u64 = 8;
const SHELL: u64 = 10;
const THINKING_AT_MOST: Duration = Duration::from_secs(4);

async fn sent(store: &Store, pane: u64) {
    let queue = store.queue().expect("the demo has a queue");
    let text = Some(Stdin::Text("Tidy up the importer".to_owned()));
    let outcome = queue.ask(last_message::send_args(pane), text).await;
    assert!(matches!(outcome, Ok(Outcome::Done(_))), "{outcome:?}");
}

async fn read(store: &Store, pane: u64) -> LastMessage {
    let transport: Arc<dyn Transport> = store.transport().expect("the demo is connected");
    transport::json(transport.as_ref(), last_message::args(pane)).await.expect("the demo has a last message")
}

mod a_message {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn puts_the_agent_to_work_and_then_back_to_idle() {
        let (store, _) = entered().await;

        sent(&store, RESTING).await;
        time::sleep(Duration::from_secs(1)).await;
        let working = status(&store, RESTING);
        time::sleep(THINKING_AT_MOST).await;

        assert_eq!(working.as_deref(), Some("working"));
        assert_eq!(status(&store, RESTING).as_deref(), Some("idle"));
    }

    #[tokio::test(start_paused = true)]
    async fn moves_the_agent_on_the_board_to_when_it_changed() {
        let (store, _) = entered().await;
        let before = store.agents().expect("a board").into_iter().find(|row| row.pane == RESTING);

        sent(&store, RESTING).await;
        time::sleep(Duration::from_secs(1)).await;
        let after = store.agents().expect("a board").into_iter().find(|row| row.pane == RESTING);

        let since = |row: Option<AgentRow>| row.and_then(|row| row.since).expect("a since");
        assert!(since(before) < since(after));
    }

    #[tokio::test(start_paused = true)]
    async fn gets_a_new_reply_each_time() {
        let (store, _) = entered().await;
        let mut replies = vec![read(&store, RESTING).await.text];

        for _ in 0..2 {
            sent(&store, RESTING).await;
            time::sleep(THINKING_AT_MOST).await;
            replies.push(read(&store, RESTING).await.text);
        }

        assert!(replies.windows(2).all(|pair| pair[0] != pair[1]), "{replies:?}");
        assert!(replies.iter().all(|reply| !reply.is_empty()));
    }

    #[tokio::test(start_paused = true)]
    async fn leaves_the_turn_open_until_the_reply() {
        let (store, _) = entered().await;
        let first = read(&store, RESTING).await;

        sent(&store, RESTING).await;
        let thinking = read(&store, RESTING).await;
        time::sleep(THINKING_AT_MOST).await;
        let replied = read(&store, RESTING).await;

        assert_eq!((first.turn_over, thinking.turn_over, replied.turn_over), (Some(true), Some(false), Some(true)));
        assert_eq!(thinking.text, first.text);
        assert!(first.written < replied.written);
    }
}

mod a_shell {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn takes_the_message_without_working() {
        let (store, _) = entered().await;

        sent(&store, SHELL).await;
        time::sleep(Duration::from_secs(1)).await;

        assert_eq!(status(&store, SHELL), None);
    }

    #[rstest]
    #[case::a_shell(SHELL)]
    #[case::no_pane(99)]
    #[tokio::test]
    async fn has_no_last_message(#[case] pane: u64) {
        let answer = transport::output(&DemoTransport::new(), last_message::args(pane)).await.expect("an answer");
        let stderr = format!("error: the demo has no answer for `{}`", last_message::args(pane).join(" "));
        assert_eq!(answer, Output { stdout: String::new(), stderr, code: 2 });
    }
}
