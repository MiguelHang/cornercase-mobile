use std::time::Duration;

use cornercase_core::cli::contract::{Kind, Missing};
use cornercase_core::error::Error;
use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::test_util::{SECRET, route};

fn json_of(value: &impl Serialize) -> Value {
    serde_json::to_value(value).expect("a view serializes")
}

mod a_machine {
    use super::*;

    fn machine() -> Machine {
        let route = route();
        Machine {
            id: MachineId(3),
            name: "studio".to_owned(),
            last_route: Some(route.address().clone()),
            routes: vec![route],
        }
    }

    #[test]
    fn names_its_routes_by_kind_and_address() {
        let expected = json!({
            "id": 3,
            "name": "studio",
            "routes": [{ "kind": "cli", "url": "ws://studio.lan:9000/", "has_token": true }],
            "last_route": "ws://studio.lan:9000/",
            "demo": false,
        });
        assert_eq!(json_of(&MachineView::from(&machine())), expected);
    }

    #[test]
    fn leaves_the_secret_out() {
        let text = serde_json::to_string(&MachineView::from(&machine())).expect("a view serializes");
        assert!(!text.contains(SECRET), "{text}");
        assert!(!text.contains(r#""token""#), "{text}");
    }

    #[test]
    fn says_a_route_whose_secret_is_missing_has_none() {
        let tokenless: Route =
            serde_json::from_value(json!({ "kind": "cli", "url": "ws://studio.lan:9000/" })).expect("a bare route");
        let expected = json!({ "kind": "cli", "url": "ws://studio.lan:9000/", "has_token": false });
        assert_eq!(json_of(&RouteView::from(&tokenless)), expected);
    }

    #[test]
    fn without_a_last_route_has_null() {
        let machine = Machine { last_route: None, ..machine() };
        assert_eq!(json_of(&MachineView::from(&machine))["last_route"], Value::Null);
    }
}

mod a_connection {
    use super::*;

    fn missing() -> Missing {
        Missing { name: "watch".to_owned(), kind: Kind::Stream, reason: None }
    }

    #[rstest]
    #[case::connecting(Connection::Connecting, json!({ "state": "connecting" }))]
    #[case::connected(
        Connection::Connected { route: route().address().clone() },
        json!({ "state": "connected", "route": "ws://studio.lan:9000/" })
    )]
    #[case::retrying(
        Connection::Retrying { r#in: Duration::from_millis(2_500), error: Error::Timeout },
        json!({
            "state": "retrying",
            "in_ms": 2_500,
            "error": { "code": "timeout", "message": "the Mac did not answer in time" },
        })
    )]
    #[case::missing(
        Connection::Missing(vec![missing()]),
        json!({ "state": "missing", "missing": [{ "name": "watch", "kind": "stream", "reason": null, "since": null }] })
    )]
    #[case::not_paired(Connection::NotPaired, json!({ "state": "not_paired" }))]
    #[case::incompatible(
        Connection::Incompatible(Error::Outdated),
        json!({ "state": "incompatible", "error": { "code": "outdated", "message": Error::Outdated.to_string() } })
    )]
    #[case::offline(Connection::Offline, json!({ "state": "offline" }))]
    fn carries_its_label_and_its_fields(#[case] connection: Connection, #[case] expected: Value) {
        assert_eq!(json_of(&ConnectionView::from(&connection)), expected);
    }

    #[rstest]
    #[case::the_cli_endpoint(Kind::Feature, "cli-endpoint")]
    #[case::the_learned_routes(Kind::Operation, "app-routes")]
    fn missing_names_the_version_that_brings_what_the_mac_lacks(#[case] kind: Kind, #[case] name: &str) {
        let lacking = Missing { name: name.to_owned(), kind, reason: None };
        let view = ConnectionView::from(&Connection::Missing(vec![lacking, missing()]));
        assert_eq!(json_of(&view)["missing"][0]["since"], json!("0.13.78"));
        assert_eq!(json_of(&view)["missing"][1]["since"], Value::Null);
    }

    #[test]
    fn missing_keeps_the_reason_the_mac_gives() {
        let disabled = Missing { name: "app-routes".to_owned(), kind: Kind::Operation, reason: Some("off".to_owned()) };
        let view = ConnectionView::from(&Connection::Missing(vec![disabled]));
        let expected = json!([{ "name": "app-routes", "kind": "operation", "reason": "off", "since": "0.13.78" }]);
        assert_eq!(json_of(&view)["missing"], expected);
    }

    #[rstest]
    #[case::a_machine(
        MachineId(3),
        Connection::Connected { route: route().address().clone() },
        json!({ "state": "connected", "route": "ws://studio.lan:9000/" })
    )]
    #[case::the_demo_connected(
        DEMO,
        Connection::Connected { route: demo::machine(DEMO).routes[0].address().clone() },
        json!({ "state": "connected", "route": "" })
    )]
    #[case::the_demo_connecting(DEMO, Connection::Connecting, json!({ "state": "connecting" }))]
    fn names_the_route_of_every_machine_but_the_demo(
        #[case] machine: MachineId,
        #[case] connection: Connection,
        #[case] expected: Value,
    ) {
        assert_eq!(json_of(&ConnectionView::of(machine, &connection)), expected);
    }

    #[test]
    fn retrying_counts_whole_seconds_in_milliseconds() {
        let connection = Connection::Retrying { r#in: Duration::from_secs(30), error: Error::Dropped };
        assert_eq!(json_of(&ConnectionView::from(&connection))["in_ms"], json!(30_000));
    }
}

mod the_connection_state {
    use super::*;

    #[test]
    fn without_a_machine_is_none() {
        assert_eq!(json_of(&ConnectionState::none()), json!({ "machine": null, "connection": { "state": "none" } }));
    }

    #[test]
    fn of_a_machine_carries_its_id_and_connection() {
        let state = ConnectionState::of(MachineId(7), &Connection::NotPaired);
        assert_eq!(json_of(&state), json!({ "machine": 7, "connection": { "state": "not_paired" } }));
    }
}

mod a_project_choice {
    use super::*;

    fn project(value: &Value) -> Project {
        serde_json::from_value(value.clone()).expect("a project of the session")
    }

    #[test]
    fn names_its_workspaces_as_the_mac_does_in_the_order_of_the_session() {
        let project = project(&json!({ "id": 5, "name": "web", "workspaces": [
            { "id": 12, "name": "main", "custom_name": null, "worktree": false },
            { "id": 9, "name": "fix-login", "custom_name": "Login", "worktree": true },
        ] }));
        let expected = json!({ "id": 5, "name": "web", "workspaces": [
            { "id": 12, "name": "main" },
            { "id": 9, "name": "Login" },
        ] });
        assert_eq!(json_of(&ProjectChoice::from(&project)), expected);
    }

    #[test]
    fn without_workspaces_lists_none() {
        let project = project(&json!({ "id": 2, "name": "api", "custom_name": "API" }));
        assert_eq!(json_of(&ProjectChoice::from(&project)), json!({ "id": 2, "name": "API", "workspaces": [] }));
    }
}
