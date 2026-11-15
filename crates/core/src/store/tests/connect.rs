use std::sync::atomic::{AtomicUsize, Ordering};

use rstest::rstest;
use serde_json::{Value, json};

use super::retry::answered;
use super::streams::{self, fine, line, served, serving};
use super::*;
use crate::cli::contract::fixtures::{BEFORE_THE_CLI, REAL};
use crate::cli::contract::{Descriptor, Kind};
use crate::cli::status::fixtures::WATCH;
use crate::store::streams::STATUS;

const OLD_CLI: &str = "error: unrecognized subcommand 'contract'";
const OLD_SERVER: &str = "Error: this server has no `contract` command";

pub(super) fn contract(json: &str) -> FakeTransport {
    FakeTransport::default().answer(CONTRACT, &[json], &[], 0)
}

pub(super) fn refusing(error: Error) -> FakeTransport {
    FakeTransport::default().fail(CONTRACT, error)
}

fn old(stderr: &str) -> FakeTransport {
    FakeTransport::default().answer(CONTRACT, &[], &[stderr], 2)
}

fn tokenless(host: &str) -> Route {
    Route::cli(route(host).address().clone(), None)
}

pub(super) fn without_notices() -> String {
    let mut contract: Value = serde_json::from_str(REAL).expect("the real contract parses");
    let features = contract["features"].as_array_mut().expect("the contract lists features");
    features.retain(|feature| feature != "notices");
    contract.to_string()
}

fn forgotten_after_a_while() -> FakeTransport {
    let calls = AtomicUsize::new(0);
    serving(fine())
        .script(CONTRACT, move |_, out| {
            let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
            async move { if first { answered(out, Some(served())).await } else { Err(Error::NotPaired) } }
        })
        .script(STATUS, |_, out| async {
            answered(out, Some(line(WATCH))).await?;
            time::sleep(Duration::from_secs(5)).await;
            Err(Error::Dropped)
        })
}

fn unreadable() -> Error {
    Descriptor::from_value(json!({"contract": "one"})).expect_err("a contract number that is not a number")
}

fn connections(expected: Connection) -> Vec<Change> {
    [Connection::Connecting, expected].map(Change::Connection).to_vec()
}

mod the_contract {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn missing_something_stops_without_asking_for_anything_else() {
        let asked = Asked::default();
        let (store, driver) = store_on(vec![route(LAN)], vec![(LAN, contract(&without_notices()))], &asked);
        let notices = Missing { name: "notices".to_owned(), kind: Kind::Feature, reason: None };
        assert_eq!(settled(&store, driver).await, connections(Connection::Missing(vec![notices])));
        assert_eq!(asked.all(), [format!("{LAN}: contract --json")]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_server_before_the_cli_stops_on_what_it_lacks() {
        let asked = Asked::default();
        let (store, driver) = store_on(vec![route(LAN)], vec![(LAN, contract(BEFORE_THE_CLI))], &asked);
        let routes = Missing { name: "app-routes".to_owned(), kind: Kind::Operation, reason: None };
        let endpoint = Missing { name: "cli-endpoint".to_owned(), kind: Kind::Feature, reason: None };
        assert_eq!(settled(&store, driver).await, connections(Connection::Missing(vec![routes, endpoint])));
        assert_eq!(asked.all(), [format!("{LAN}: contract --json")]);
    }

    #[rstest]
    #[case::from_an_old_cli(vec![(LAN, old(OLD_CLI))], Error::Outdated)]
    #[case::from_an_old_server(vec![(LAN, old(OLD_SERVER))], Error::Outdated)]
    #[case::beside_an_unreachable_route(vec![(TAILNET, old(OLD_CLI))], Error::Outdated)]
    #[case::without_a_number(vec![(LAN, contract("{}"))], Error::Outdated)]
    #[case::newer_than_the_app(vec![(LAN, contract(r#"{"contract":2}"#))], Error::ContractTooNew { server: 2, app: 1 })]
    #[case::of_another_shape(vec![(LAN, contract(r#"{"contract":"one"}"#))], unreadable())]
    #[tokio::test(start_paused = true)]
    async fn that_does_not_fit_stops_as_incompatible(
        #[case] hosts: Vec<(&'static str, FakeTransport)>,
        #[case] error: Error,
    ) {
        let (store, driver) = store_on(vec![route(LAN), route(TAILNET)], hosts, &Asked::default());
        assert_eq!(settled(&store, driver).await, connections(Connection::Incompatible(error)));
    }
}

mod the_token {
    use super::*;

    #[rstest]
    #[case::on_every_route(vec![route(LAN), route(TAILNET)])]
    #[case::refused_on_one_route_and_absent_on_the_other(vec![route(LAN), tokenless(TAILNET)])]
    #[case::absent_on_every_route(vec![tokenless(LAN), tokenless(TAILNET)])]
    #[case::of_a_machine_without_routes(Vec::new())]
    #[tokio::test(start_paused = true)]
    async fn refused_or_absent_stops_as_unauthorized(#[case] routes: Vec<Route>) {
        let hosts = vec![(LAN, refusing(Error::Unauthorized)), (TAILNET, refusing(Error::Unauthorized))];
        let (store, driver) = store_on(routes, hosts, &Asked::default());
        assert_eq!(settled(&store, driver).await, connections(Connection::Unauthorized));
    }

    #[tokio::test(start_paused = true)]
    async fn refused_on_one_route_retries_with_what_the_other_said() {
        let hosts = vec![(LAN, refusing(Error::Unauthorized)), (TAILNET, refusing(Error::Timeout))];
        let (store, driver) = store_on(vec![route(LAN), route(TAILNET)], hosts, &Asked::default());
        let retrying = Connection::Retrying { r#in: Duration::from_secs(1), error: Error::Timeout };
        assert_eq!(settled(&store, driver).await[..2], connections(retrying));
    }
}

mod the_pairing {
    use super::*;

    #[rstest]
    #[case::on_every_route(vec![route(LAN), route(TAILNET)], Error::NotPaired)]
    #[case::beside_an_unreachable_route(vec![route(LAN), route(TAILNET)], Error::Timeout)]
    #[case::beside_a_dropped_route(vec![route(LAN), route(TAILNET)], Error::Dropped)]
    #[case::beside_a_refused_token(vec![route(LAN), route(TAILNET)], Error::Unauthorized)]
    #[case::beside_a_route_without_a_secret(vec![route(LAN), tokenless(TAILNET)], Error::Timeout)]
    #[tokio::test(start_paused = true)]
    async fn forgotten_on_one_route_stops_as_not_paired(#[case] routes: Vec<Route>, #[case] other: Error) {
        let hosts = vec![(LAN, refusing(Error::NotPaired)), (TAILNET, refusing(other))];
        let (store, driver) = store_on(routes, hosts, &Asked::default());
        assert_eq!(settled(&store, driver).await, connections(Connection::NotPaired));
    }

    #[tokio::test(start_paused = true)]
    async fn forgotten_while_connected_stops_after_the_drop_instead_of_retrying() {
        let (store, driver) = store_on(vec![route(LAN)], vec![(LAN, forgotten_after_a_while())], &Asked::default());
        let states = streams::connections(&settled(&store, driver).await);
        let expected = [
            Connection::Connecting,
            Connection::Connected { route: route(LAN).address().clone() },
            Connection::Retrying { r#in: Duration::from_secs(1), error: Error::Dropped },
            Connection::Connecting,
            Connection::NotPaired,
        ];
        assert_eq!(states, expected);
        assert_eq!(store.connection(), Connection::NotPaired);
    }
}
