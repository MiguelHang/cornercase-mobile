use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::cli::contract::fixtures::REAL;

const NO_OPERATIONS: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/contract/no-operations.json"));

fn real() -> Value {
    serde_json::from_str(REAL).expect("the fixture is JSON")
}

mod reading {
    use super::*;

    #[test]
    fn reads_what_cornercase_0_13_19_says() {
        let descriptor = Descriptor::from_value(real()).expect("the real contract");
        assert_eq!(descriptor.version, "0.13.19");
        assert_eq!(descriptor.contract, 1);
        assert_eq!(descriptor.boot, "28715-1794075132409911000");
        assert_eq!(descriptor.operations.len(), 124);
        let status = Operation { name: "status".to_owned(), available: true, reason: None };
        assert_eq!(descriptor.operations[0], status);
        assert!(descriptor.features.iter().any(|feature| feature == "boot"));
        assert!(descriptor.streams.contains(&Stream { name: "events".to_owned(), version: 1 }));
    }

    #[test]
    fn reads_the_same_from_text() {
        let descriptor: Descriptor = REAL.parse().expect("the real contract");
        assert_eq!(descriptor, Descriptor::from_value(real()).expect("the real contract"));
    }

    #[test]
    fn ignores_keys_it_does_not_know() {
        let value = json!({
            "contract": 1,
            "later": true,
            "operations": [{ "name": "status", "available": false, "reason": "busy", "since": 3 }],
            "streams": [{ "name": "events", "version": 1, "fields": [] }],
        });
        let descriptor = Descriptor::from_value(value).expect("a descriptor");
        let status = Operation { name: "status".to_owned(), available: false, reason: Some("busy".to_owned()) };
        assert_eq!(descriptor.operations, [status]);
        assert_eq!(descriptor.streams, [Stream { name: "events".to_owned(), version: 1 }]);
    }

    #[test]
    fn leaves_what_is_missing_empty() {
        let value = json!({ "contract": 1, "operations": [{}], "streams": [{}] });
        let descriptor = Descriptor::from_value(value).expect("a descriptor");
        let expected = Descriptor {
            contract: 1,
            operations: vec![Operation::default()],
            streams: vec![Stream::default()],
            ..Descriptor::default()
        };
        assert_eq!(descriptor, expected);
    }
}

mod shape {
    use super::*;

    #[test]
    fn says_where_the_text_has_the_wrong_shape_but_not_what_it_says() {
        let error =
            "{\"contract\": 1,\n\"boot\": [\"secret-token\"]}".parse::<Descriptor>().expect_err("a wrong shape");
        let Error::Json(reason) = error else { panic!("not a JSON error: {error:?}") };
        assert!(reason.starts_with("not the expected shape at line 2 column"), "{reason}");
        assert!(!reason.contains("secret-token"), "{reason}");
    }

    #[test]
    fn says_a_value_has_the_wrong_shape_but_not_what_it_says() {
        let error = Descriptor::from_value(json!({ "contract": "secret-token" })).expect_err("a wrong shape");
        let Error::Json(reason) = error else { panic!("not a JSON error: {error:?}") };
        assert!(reason.starts_with("not the expected shape at line"), "{reason}");
        assert!(!reason.contains("secret-token"), "{reason}");
    }

    #[rstest]
    #[case::not_json("contract 1", "not JSON at line 1 column 1")]
    #[case::cut_short("{\"contract\": 1", "the JSON ends too early at line 1 column 14")]
    fn says_why_the_text_is_not_a_descriptor(#[case] text: &str, #[case] reason: &str) {
        assert_eq!(text.parse::<Descriptor>(), Err(Error::Json(reason.to_owned())));
    }
}

mod contract {
    use super::*;

    #[rstest]
    #[case::absent(json!({ "version": "0.9.0" }), Err(Error::Outdated))]
    #[case::zero(json!({ "contract": 0 }), Err(Error::Outdated))]
    #[case::current(json!({ "contract": 1 }), Ok(1))]
    #[case::newer(json!({ "contract": 2 }), Err(Error::ContractTooNew { server: 2, app: 1 }))]
    fn accepts_only_the_contracts_this_app_knows(#[case] value: Value, #[case] expected: Result<u32, Error>) {
        assert_eq!(Descriptor::from_value(value).map(|descriptor| descriptor.contract), expected);
    }

    #[rstest]
    #[case::zero("{\"contract\": 0}", Err(Error::Outdated))]
    #[case::current("{\"contract\": 1}", Ok(1))]
    #[case::newer("{\"contract\": 2}", Err(Error::ContractTooNew { server: 2, app: 1 }))]
    fn checks_the_contract_of_text_too(#[case] text: &str, #[case] expected: Result<u32, Error>) {
        assert_eq!(text.parse::<Descriptor>().map(|descriptor| descriptor.contract), expected);
    }
}

mod missing {
    use super::*;

    fn real_but(edit: impl FnOnce(&mut Value)) -> Descriptor {
        let mut value = real();
        edit(&mut value);
        Descriptor::from_value(value).expect("a descriptor")
    }

    fn entry<'a>(value: &'a mut Value, list: &str, name: &str) -> &'a mut Value {
        let entries = value[list].as_array_mut().expect("a list");
        entries.iter_mut().find(|entry| entry["name"] == name).expect("the entry")
    }

    fn unavailable(value: &mut Value, name: &str, reason: &str) {
        *entry(value, "operations", name) = json!({ "name": name, "available": false, "reason": reason });
    }

    fn without(value: &mut Value, list: &str, name: &str) {
        value[list].as_array_mut().expect("a list").retain(|entry| entry != name && entry["name"] != name);
    }

    fn one(name: &str, kind: Kind, reason: Option<&str>) -> Missing {
        Missing { name: name.to_owned(), kind, reason: reason.map(str::to_owned) }
    }

    #[test]
    fn nothing_is_missing_from_cornercase_0_13_19() {
        assert_eq!(missing(&Descriptor::from_value(real()).expect("the real contract")), []);
    }

    #[test]
    fn ignores_updates_a_dev_build_cannot_do() {
        let descriptor = real_but(|value| {
            unavailable(value, "update-check", "a development build");
            unavailable(value, "update-install", "a development build");
        });
        assert_eq!(missing(&descriptor), []);
    }

    #[test]
    fn lists_every_required_operation_in_order_when_there_are_none() {
        let descriptor: Descriptor = NO_OPERATIONS.parse().expect("a descriptor");
        let expected: Vec<Missing> = REQUIRED.operations.iter().map(|name| one(name, Kind::Operation, None)).collect();
        assert_eq!(missing(&descriptor), expected);
    }

    #[test]
    fn says_why_the_server_cannot_do_an_operation() {
        let descriptor = real_but(|value| unavailable(value, "send", "no project is open"));
        assert_eq!(missing(&descriptor), [one("send", Kind::Operation, Some("no project is open"))]);
    }

    #[test]
    fn does_not_need_a_server_that_makes_workspaces() {
        let descriptor = real_but(|value| without(value, "operations", "new-workspace"));
        assert_eq!(missing(&descriptor), []);
    }

    #[test]
    fn reports_a_server_that_cannot_list_the_agents() {
        let descriptor = real_but(|value| without(value, "operations", "account-list"));
        assert_eq!(missing(&descriptor), [one("account-list", Kind::Operation, None)]);
    }

    #[test]
    fn reports_a_server_that_cannot_mark_a_pane_as_reviewed() {
        let descriptor = real_but(|value| without(value, "operations", "mark"));
        assert_eq!(missing(&descriptor), [one("mark", Kind::Operation, None)]);
    }

    #[rstest]
    #[case::the_flows_of_a_project("flow-list")]
    #[case::a_flow_run("flow-run")]
    fn reports_a_server_that_cannot_start_a_flow(#[case] name: &str) {
        let descriptor = real_but(|value| without(value, "operations", name));
        assert_eq!(missing(&descriptor), [one(name, Kind::Operation, None)]);
    }

    #[rstest]
    #[case::views("views")]
    #[case::notices("notices")]
    fn reports_a_feature_the_server_lacks(#[case] name: &str) {
        let descriptor = real_but(|value| without(value, "features", name));
        assert_eq!(missing(&descriptor), [one(name, Kind::Feature, None)]);
    }

    #[rstest]
    #[case::absent(|value: &mut Value| without(value, "streams", "surface"))]
    #[case::another_version(|value: &mut Value| entry(value, "streams", "surface")["version"] = json!(2))]
    fn reports_a_stream_it_cannot_read(#[case] edit: fn(&mut Value)) {
        assert_eq!(missing(&real_but(edit)), [one("surface", Kind::Stream, None)]);
    }

    #[test]
    fn lists_operations_then_features_then_streams() {
        let descriptor = real_but(|value| {
            without(value, "streams", "events");
            without(value, "features", "boot");
            without(value, "operations", "files-search");
            without(value, "operations", "status");
        });
        let expected = [
            one("status", Kind::Operation, None),
            one("files-search", Kind::Operation, None),
            one("boot", Kind::Feature, None),
            one("events", Kind::Stream, None),
        ];
        assert_eq!(missing(&descriptor), expected);
    }

    #[rstest]
    #[case::operation(one("send", Kind::Operation, Some("busy")), json!({ "name": "send", "kind": "operation", "reason": "busy" }))]
    #[case::feature(one("boot", Kind::Feature, None), json!({ "name": "boot", "kind": "feature", "reason": null }))]
    #[case::stream(one("view", Kind::Stream, None), json!({ "name": "view", "kind": "stream", "reason": null }))]
    fn writes_its_name_kind_and_reason(#[case] missing: Missing, #[case] expected: Value) {
        assert_eq!(serde_json::to_value(missing).expect("JSON"), expected);
    }
}

mod outdated {
    use super::*;

    const OLD_SERVER: &str = "Error: the running cornercase server (0.12.0) has no `contract` command; update cornercase and restart the server\n";
    const OLD_CLI: &str = "error: unrecognized subcommand 'contract'\n\nUsage: cornercase [OPTIONS] [COMMAND]\n\nFor more information, try '--help'.\n";
    const BRIDGE: &str = "--remote is not allowed through the bridge: the bridge runs commands on this Mac";

    fn failed(code: i32, stderr: &str) -> Error {
        Error::Failed { code, stderr: stderr.to_owned() }
    }

    #[rstest]
    #[case::server_without_the_command(failed(1, OLD_SERVER))]
    #[case::cli_without_the_subcommand(failed(2, OLD_CLI))]
    fn says_cornercase_on_the_mac_is_too_old(#[case] error: Error) {
        assert_eq!(outdated(error), Error::Outdated);
    }

    #[rstest]
    #[case::refused_by_the_bridge(failed(2, BRIDGE))]
    #[case::another_failure(failed(1, "Error: no project at /tmp/x"))]
    #[case::not_a_failure(Error::Timeout)]
    fn leaves_any_other_error_as_it_is(#[case] error: Error) {
        assert_eq!(outdated(error.clone()), error);
    }
}
