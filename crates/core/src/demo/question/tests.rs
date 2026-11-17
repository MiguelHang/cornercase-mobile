use std::time::Duration;

use rstest::rstest;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::time;

use crate::cli::keys;
use crate::cli::surface::{Size, on_tab};
use crate::demo::DemoTransport;
use crate::demo::tests::{entered, status};
use crate::queue::Outcome;
use crate::store::Store;
use crate::terminal::asking::{Asked, Asking};
use crate::terminal::question::{Block, Card};
use crate::transport::{self, Output};

const TAB: u64 = 12;
const ASKING: u64 = 13;
const SIZE: Size = Size { cols: 80, rows: 24 };
const WORKING_AT_MOST: Duration = Duration::from_secs(4);

fn asked(store: &Store) -> UnboundedReceiver<Asked> {
    let (said, heard) = mpsc::unbounded_channel();
    let asking = Asking { tab: TAB, pane: ASKING, size: SIZE };
    tokio::spawn(asking.run(store.clone(), move |asked| drop(said.send(asked))));
    heard
}

fn shown(heard: &mut UnboundedReceiver<Asked>) -> Option<Card> {
    let mut card = None;
    while let Ok(asked) = heard.try_recv() {
        let Asked::Question(question) = asked else { panic!("the question failed: {asked:?}") };
        card = question.card;
    }
    card
}

async fn pressed(store: &Store, key: &str) {
    pressed_in(store, ASKING, key).await;
}

async fn pressed_in(store: &Store, pane: u64, key: &str) {
    let queue = store.queue().expect("the demo has a queue");
    let outcome = queue.ask(keys::args(pane, &[key]).expect("a known key"), None).await;
    assert!(matches!(outcome, Ok(Outcome::Done(_))), "{outcome:?}");
}

async fn opened() -> (Store, UnboundedReceiver<Asked>) {
    let (store, _) = entered().await;
    let heard = asked(&store);
    time::sleep(Duration::from_secs(1)).await;
    (store, heard)
}

mod the_question {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn is_read_as_a_card_with_its_options() {
        let (_store, mut heard) = opened().await;

        let card = shown(&mut heard).expect("the agent asks a question");

        let labels: Vec<&str> = card.options.iter().map(|option| option.label.as_str()).collect();
        assert_eq!(labels, ["Round each line", "Round the total", "Keep both"]);
        assert!(card.options.iter().all(|option| option.detail.is_some()));
        assert_eq!(card.options.iter().filter(|option| option.selected).count(), 1);
        assert_eq!(card.tabs.iter().map(|tab| tab.label.as_str()).collect::<Vec<_>>(), ["Rounding"]);
        let asked = "How should the cart round its totals when several discounts apply?";
        assert_eq!(card.blocks, [Block::Text(asked.to_owned())]);
        assert_eq!(card.help.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn goes_away_once_answered_and_the_agent_works_and_ends_done() {
        let (store, mut heard) = opened().await;
        let asking = status(&store, ASKING);
        shown(&mut heard);

        pressed(&store, "2").await;
        time::sleep(Duration::from_secs(1)).await;
        let (card, working) = (shown(&mut heard), status(&store, ASKING));
        time::sleep(WORKING_AT_MOST).await;

        assert_eq!(asking.as_deref(), Some("waiting"));
        assert_eq!(card, None);
        assert_eq!(working.as_deref(), Some("working"));
        assert_eq!(status(&store, ASKING).as_deref(), Some("done"));
    }

    #[rstest]
    #[case::a_key_that_is_not_an_option("enter")]
    #[case::a_number_past_the_options("9")]
    #[case::the_number_just_past_the_options("4")]
    #[case::zero("0")]
    #[tokio::test(start_paused = true)]
    async fn stays_for_any_other_key(#[case] key: &str) {
        let (store, mut heard) = opened().await;
        shown(&mut heard);

        pressed(&store, key).await;
        time::sleep(WORKING_AT_MOST).await;

        assert!(heard.is_empty(), "the question changed");
        assert_eq!(status(&store, ASKING).as_deref(), Some("waiting"));
    }
}

mod the_surface {
    use super::*;

    #[tokio::test]
    async fn of_a_tab_that_is_not_there_fails() {
        let argv: Vec<String> =
            ["surface".to_owned()].into_iter().chain(on_tab(99, SIZE)).chain(["--patches".to_owned()]).collect();
        let answer = transport::output(&DemoTransport::new(), argv).await.expect("the demo answers");
        let stderr = "error: no tab 99".to_owned();
        assert_eq!(answer, Output { stdout: String::new(), stderr, code: 1 });
    }

    #[tokio::test(start_paused = true)]
    async fn stays_for_a_key_pressed_in_another_pane() {
        let (store, mut heard) = opened().await;
        shown(&mut heard);

        pressed_in(&store, 4, "1").await;
        time::sleep(WORKING_AT_MOST).await;

        assert!(heard.is_empty(), "the question changed");
        assert_eq!(status(&store, ASKING).as_deref(), Some("waiting"));
    }
}
