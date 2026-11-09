use rstest::rstest;
use serde_json::{Value, json};

use super::*;

pub(crate) const REAL: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/contract/0.13.19.json"));

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
