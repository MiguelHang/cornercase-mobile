use super::*;

mod argv {
    use super::*;

    #[test]
    fn marks_one_pane_as_reviewed() {
        assert_eq!(reviewed_args(543), ["mark", "--pane", "543", "--reviewed"]);
    }

    #[test]
    fn writes_the_whole_pane_number() {
        assert_eq!(reviewed_args(u64::MAX)[2], u64::MAX.to_string());
    }
}
