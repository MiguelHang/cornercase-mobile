use std::time::Duration;

use futures_util::FutureExt;
use rstest::rstest;
use tokio::time;

use super::*;
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

mod the_contract {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn is_asked_for_and_accepted_by_the_store() {
        let demo = DemoTransport::new();
        let factory = move |_: &Route| Ok(Arc::new(demo.clone()) as Arc<dyn Transport>);
        let (store, driver) = Store::new(demo_machine(), factory, PickConfig::default());
        let mut changes = store.changes();
        tokio::spawn(driver);
        time::sleep(Duration::from_secs(60)).await;
        let seen: Vec<Change> = std::iter::from_fn(|| changes.next().now_or_never().flatten()).collect();
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
