use std::time::Duration;

use cornercase_core::cli::contract::{Kind, Missing};
use cornercase_core::error::Error;
use rstest::rstest;
use serde_json::{Value, json};

use super::*;

const URL: &str = "ws://studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA";

fn route() -> Route {
    Route::parse(URL).expect("a route with a token")
}

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
            "routes": [{ "kind": "bridge", "url": "ws://studio.lan:9000/" }],
            "last_route": "ws://studio.lan:9000/",
        });
        assert_eq!(json_of(&MachineView::from(&machine())), expected);
    }

    #[test]
    fn leaves_the_token_out() {
        let text = serde_json::to_string(&MachineView::from(&machine())).expect("a view serializes");
        assert!(!text.contains("c3R1ZGlvLXNlY3JldA"), "{text}");
        assert!(!text.contains("token"), "{text}");
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
            "error": { "code": "timeout", "message": "the bridge did not answer in time" },
        })
    )]
    #[case::missing(
        Connection::Missing(vec![missing()]),
        json!({ "state": "missing", "missing": [{ "name": "watch", "kind": "stream", "reason": null }] })
    )]
    #[case::unauthorized(Connection::Unauthorized, json!({ "state": "unauthorized" }))]
    #[case::incompatible(
        Connection::Incompatible(Error::Outdated),
        json!({ "state": "incompatible", "error": { "code": "outdated", "message": Error::Outdated.to_string() } })
    )]
    fn carries_its_label_and_its_fields(#[case] connection: Connection, #[case] expected: Value) {
        assert_eq!(json_of(&ConnectionView::from(&connection)), expected);
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
        let state = ConnectionState::of(MachineId(7), &Connection::Unauthorized);
        assert_eq!(json_of(&state), json!({ "machine": 7, "connection": { "state": "unauthorized" } }));
    }
}
