use std::time::{Duration, SystemTime};

use futures_util::FutureExt;
use rstest::rstest;
use tokio::time;

use super::clock::iso;
use super::*;
use crate::board::AgentRow;
use crate::cli::contract::{self, Descriptor};
use crate::machines::pick::PickConfig;
use crate::machines::route::{Address, Route, Token};
use crate::machines::{Machine, MachineId};
use crate::store::{Change, Connection, Store};
use crate::transport::{self, Output};

fn demo_machine() -> Machine {
    let address = Address { host: "demo".to_owned(), port: 0 };
    let route = Route::cli(address, Some(Token::new("demo".to_owned())));
    Machine { id: MachineId(1), name: "Demo Mac".to_owned(), routes: vec![route], last_route: None }
}

fn owned(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|&word| word.to_owned()).collect()
}

fn refused(connection: &Connection) -> bool {
    matches!(connection, Connection::Missing(_) | Connection::NotPaired | Connection::Incompatible(_))
}

pub(super) async fn entered() -> (Store, Vec<Change>) {
    let demo = DemoTransport::new();
    let factory = move |_: &Route| Ok(Arc::new(demo.clone()) as Arc<dyn Transport>);
    let (store, driver) = Store::new(demo_machine(), factory, PickConfig::default());
    let mut changes = store.changes();
    tokio::spawn(driver);
    time::sleep(Duration::from_secs(60)).await;
    let seen = std::iter::from_fn(|| changes.next().now_or_never().flatten()).collect();
    (store, seen)
}

pub(super) fn status(store: &Store, pane: u64) -> Option<String> {
    let rows = store.agents().expect("the demo shows a board");
    rows.into_iter().find(|row| row.pane == pane).and_then(|row| row.status)
}

async fn board() -> Vec<AgentRow> {
    entered().await.0.agents().expect("the demo shows a board")
}

mod the_contract {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn is_asked_for_and_accepted_by_the_store() {
        let (store, seen) = entered().await;
        assert_eq!(store.boot().as_deref(), Some("demo"));
        assert!(!seen.iter().any(|change| matches!(change, Change::Connection(connection) if refused(connection))));
    }

    #[tokio::test]
    async fn names_everything_the_app_requires() {
        let read = transport::json(&DemoTransport::new(), owned(&["contract", "--json"])).await;
        let descriptor = Descriptor::from_value(read.expect("the demo prints its contract")).expect("it parses");
        assert_eq!(contract::missing(&descriptor), []);
    }
}

mod the_streams {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn connect_once_and_stay_open() {
        let (store, seen) = entered().await;
        let connections: Vec<Connection> = seen
            .into_iter()
            .filter_map(|change| match change {
                Change::Connection(connection) => Some(connection),
                _ => None,
            })
            .collect();
        let route = Address { host: "demo".to_owned(), port: 0 };
        assert_eq!(connections, [Connection::Connecting, Connection::Connected { route }]);
        assert_eq!(store.session().map(|session| session.boot).as_deref(), Some("demo"));
    }

    #[tokio::test(start_paused = true)]
    async fn follow_the_app_of_the_demo() {
        let (store, _) = entered().await;
        let app = store.app().expect("the demo follows the app");
        assert_eq!(app["boot"], "demo");
    }
}

mod the_board {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn has_agents_working_asking_and_done() {
        let rows = board().await;
        let statuses: Vec<&str> = rows.iter().filter_map(|row| row.status.as_deref()).collect();
        for status in ["working", "waiting", "done", "idle"] {
            assert!(statuses.contains(&status), "no agent is {status} in {statuses:?}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn nests_subagents_under_their_parent() {
        let rows = board().await;
        let parent = rows.iter().find(|row| row.depth == 0 && row.pane == 4).expect("the lead agent");
        let children: Vec<u64> =
            rows.iter().filter(|row| row.parent == Some(parent.pane)).map(|row| row.pane).collect();
        assert_eq!(children.len(), 2);
        assert!(rows.iter().filter(|row| children.contains(&row.pane)).all(|row| row.depth == 1));
    }

    #[tokio::test(start_paused = true)]
    async fn dates_every_agent_shortly_before_entering() {
        let rows = board().await;
        let (earliest, latest) = (iso(SystemTime::now() - Duration::from_secs(3_600)), iso(SystemTime::now()));
        for row in &rows {
            let since = row.since.as_deref().expect("every agent says since when");
            assert!(earliest.as_str() <= since && since <= latest.as_str(), "{since} is not within the last hour");
        }
    }
}

mod anything_else {
    use super::*;

    #[rstest]
    #[case::an_unknown_command(&["issues", "--json"])]
    #[case::the_contract_without_json(&["contract"])]
    #[case::nothing(&[])]
    #[tokio::test]
    async fn fails_naming_what_was_asked(#[case] argv: &[&str]) {
        let answer = transport::output(&DemoTransport::new(), owned(argv)).await.expect("the demo answers");
        let stderr = format!("error: the demo has no answer for `{}`", argv.join(" "));
        assert_eq!(answer, Output { stdout: String::new(), stderr, code: 2 });
    }
}

mod privacy;
