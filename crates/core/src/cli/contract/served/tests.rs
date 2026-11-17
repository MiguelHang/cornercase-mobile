use rstest::rstest;

use super::*;
use crate::cli::contract::boot::Boot;
use crate::cli::contract::{Descriptor, missing};

const BOOT: &str = "4242-1700000000";

fn read(value: Value) -> Descriptor {
    Descriptor::from_value(value).expect("a descriptor")
}

#[test]
fn lacks_nothing_this_app_requires() {
    assert_eq!(missing(&read(descriptor(BOOT))), []);
}

#[test]
fn speaks_the_contract_this_app_knows_with_the_boot_it_is_given() {
    let read = read(descriptor(BOOT));
    assert_eq!(read.contract, CONTRACT);
    assert_eq!(read.boot(), Boot(BOOT.to_owned()));
}

#[test]
fn reads_the_same_as_text() {
    let text = descriptor(BOOT).to_string();
    assert_eq!(text.parse::<Descriptor>(), Ok(read(descriptor(BOOT))));
}

#[test]
fn says_it_is_the_newest_version_that_brought_something_required() {
    assert_eq!(read(descriptor(BOOT)).version, "0.13.78");
}

#[test]
fn offers_only_what_is_required() {
    let read = read(descriptor(BOOT));
    let operations: Vec<&str> = read.operations.iter().map(|operation| operation.name.as_str()).collect();
    let streams: Vec<(&str, u32)> = read.streams.iter().map(|stream| (stream.name.as_str(), stream.version)).collect();
    assert_eq!(operations, REQUIRED.operations);
    assert_eq!(read.features, REQUIRED.features);
    assert_eq!(streams, REQUIRED.streams);
}

#[test]
fn grows_with_what_is_required() {
    let more = Required {
        operations: &["status", "teleport"],
        features: &["boot", "wormholes"],
        streams: &[("events", 1), ("dreams", 3)],
    };
    let read = read(more.described("9.0.0", BOOT));
    assert_eq!(more.missing(&read), []);
    assert_eq!(read.version, "9.0.0");
}

#[rstest]
#[case::by_number_not_by_text(&["0.9.0", "0.13.78"], "0.13.78")]
#[case::by_the_patch(&["0.13.78", "0.13.8"], "0.13.78")]
#[case::by_the_major(&["1.0.0", "0.99.99"], "1.0.0")]
#[case::alone(&["0.13.78"], "0.13.78")]
#[case::none(&[], "")]
fn picks_the_newest_version(#[case] versions: &[&str], #[case] expected: &str) {
    assert_eq!(newest(versions.iter().copied()), expected);
}
