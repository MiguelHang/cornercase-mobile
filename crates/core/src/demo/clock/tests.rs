use std::time::Duration;

use rstest::rstest;

use super::*;

#[rstest]
#[case::the_epoch(0, "1970-01-01T00:00:00.000Z")]
#[case::a_leap_day_of_a_century(951_782_400_000, "2000-02-29T00:00:00.000Z")]
#[case::a_leap_day_with_millis(1_709_164_800_500, "2024-02-29T00:00:00.500Z")]
#[case::an_evening(1_794_260_990_750, "2026-11-09T21:49:50.750Z")]
fn iso_writes_the_utc_time_the_app_parses(#[case] millis: u64, #[case] written: &str) {
    assert_eq!(iso(UNIX_EPOCH + Duration::from_millis(millis)), written);
}

#[test]
fn iso_writes_the_epoch_for_a_time_before_it() {
    assert_eq!(iso(UNIX_EPOCH - Duration::from_secs(1)), "1970-01-01T00:00:00.000Z");
}
