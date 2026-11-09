use super::*;

mod waits {
    use super::*;

    #[test]
    fn double_from_one_second_and_stay_at_fifteen() {
        let mut backoff = Backoff::default();
        let waits: Vec<u64> = (0..8).map(|_| backoff.failed().as_secs()).collect();
        assert_eq!(waits, [1, 2, 4, 8, 15, 15, 15, 15]);
    }
}

mod reset {
    use super::*;

    #[test]
    fn starts_again_from_one_second() {
        let mut backoff = Backoff::default();
        let before: Vec<u64> = (0..3).map(|_| backoff.failed().as_secs()).collect();
        backoff.reset();
        let after: Vec<u64> = (0..3).map(|_| backoff.failed().as_secs()).collect();
        assert_eq!([before, after], [[1, 2, 4], [1, 2, 4]]);
    }
}
