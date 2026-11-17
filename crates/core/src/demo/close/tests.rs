use std::time::Duration;

use tokio::time;

use crate::cli::close::{self, AgentClosing};
use crate::cli::mark;
use crate::cli::surface::{self, Size};
use crate::demo::chat::DONE;
use crate::demo::tests::{entered, status};
use crate::demo::world::World;
use crate::error::Error;
use crate::queue::Outcome;
use crate::store::Store;
use crate::transport;

const ROOT: u64 = 4;
const SUBAGENTS: [u64; 2] = [6, 8];
const FINISHED: u64 = 17;

async fn closing(store: &Store, pane: u64, boot: &str, check: bool) -> Result<AgentClosing, Error> {
    let queue = store.queue().expect("the demo has a queue");
    let Outcome::Done(text) = queue.ask(close::agent_args(pane, boot, check), None).await? else {
        panic!("the demo skipped the close")
    };
    Ok(serde_json::from_str(&text).expect("the demo answers an agent closing"))
}

async fn closed(store: &Store, pane: u64) -> AgentClosing {
    let closing = closing(store, pane, "demo", false).await.expect("the demo closes the agent");
    time::sleep(Duration::from_secs(1)).await;
    closing
}

fn on_board(store: &Store) -> Vec<u64> {
    store.agents().expect("the demo shows a board").into_iter().map(|row| row.pane).collect()
}

mod a_check {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn names_the_agent_and_its_subagents_without_closing_them() {
        let (store, _) = entered().await;

        let checked = closing(&store, ROOT, "demo", true).await.expect("the demo checks the close");
        time::sleep(Duration::from_secs(1)).await;

        let stopped: Vec<u64> = checked.stops.iter().map(|stop| stop.pane).collect();
        assert_eq!((checked.closed, checked.agent.as_str(), stopped), (false, "claude", vec![ROOT, 6, 8]));
        assert_eq!((checked.tabs, checked.workspace, checked.project), (vec![3, 5, 7], 2, 1));
        assert!(on_board(&store).contains(&ROOT));
    }

    #[tokio::test(start_paused = true)]
    async fn is_refused_for_another_boot() {
        let (store, _) = entered().await;
        let refused = closing(&store, ROOT, "earlier-boot", true).await;
        assert!(matches!(refused, Err(Error::Failed { code: 2, .. })), "{refused:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn is_refused_for_a_pane_without_an_agent() {
        let (store, _) = entered().await;
        let refused = closing(&store, 10, "demo", true).await;
        assert!(matches!(refused, Err(Error::Failed { code: 2, .. })), "{refused:?}");
    }
}

mod a_close {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn takes_the_agent_off_the_board() {
        let (store, _) = entered().await;
        let closing = closed(&store, FINISHED).await;
        assert!(closing.closed);
        assert!(!on_board(&store).contains(&FINISHED), "{:?}", on_board(&store));
        assert!(on_board(&store).contains(&13));
    }

    #[tokio::test(start_paused = true)]
    async fn takes_the_subagents_with_it() {
        let (store, _) = entered().await;
        closed(&store, ROOT).await;
        let left = on_board(&store);
        assert!([ROOT].iter().chain(&SUBAGENTS).all(|pane| !left.contains(pane)), "{left:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn leaves_no_tab_to_draw() {
        let (store, _) = entered().await;
        closed(&store, FINISHED).await;
        let transport = store.transport().expect("the demo is connected");
        let tab = surface::on_tab(16, Size { cols: 80, rows: 24 });
        let argv: Vec<String> = ["surface".to_owned()].into_iter().chain(tab).chain(["--patches".to_owned()]).collect();
        let drawn = transport::output(transport.as_ref(), argv).await.expect("the demo answers");
        assert_eq!((drawn.code, drawn.stderr.as_str()), (1, "error: no tab 16"));
    }

    #[tokio::test(start_paused = true)]
    async fn forgets_the_turn_and_the_terminal_of_its_panes() {
        let mut world = World::default();
        world.worked(FINISHED, DONE);
        assert!(world.next_due().is_some());

        let closing = world.closing(FINISHED).expect("the demo can close the agent");
        world.closed(&closing.tabs);

        assert_eq!(world.next_due(), None);
        assert!(!world.printed.contains_key(&FINISHED));
        assert!(world.printed.contains_key(&ROOT));
    }
}

mod a_review {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn turns_a_finished_agent_idle() {
        let (store, _) = entered().await;
        let queue = store.queue().expect("the demo has a queue");

        queue.ask(mark::reviewed_args(FINISHED), None).await.expect("the demo marks the agent reviewed");
        time::sleep(Duration::from_secs(1)).await;

        assert_eq!(status(&store, FINISHED).as_deref(), Some("idle"));
    }

    #[tokio::test(start_paused = true)]
    async fn is_not_reviewed_by_any_other_mark() {
        let (store, _) = entered().await;
        let queue = store.queue().expect("the demo has a queue");
        let argv = ["mark", "--pane", "17", "--seen"].map(str::to_owned).to_vec();

        queue.ask(argv, None).await.expect("the demo takes the mark");
        time::sleep(Duration::from_secs(1)).await;

        assert_eq!(status(&store, FINISHED).as_deref(), Some("done"));
    }

    #[tokio::test(start_paused = true)]
    async fn leaves_a_working_agent_working() {
        let (store, _) = entered().await;
        let queue = store.queue().expect("the demo has a queue");

        queue.ask(mark::reviewed_args(ROOT), None).await.expect("the demo marks the agent reviewed");
        time::sleep(Duration::from_secs(1)).await;

        assert_eq!(status(&store, ROOT).as_deref(), Some("working"));
    }
}
