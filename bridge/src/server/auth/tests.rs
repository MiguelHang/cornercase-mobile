use rstest::rstest;

use super::*;

const TOKEN: &str = "s3cret";

mod check {
    use super::*;

    #[rstest]
    #[case::the_right_token("/?token=s3cret")]
    #[case::among_other_keys("/?a=1&token=s3cret&b")]
    fn lets_the_right_token_through(#[case] uri: &str) {
        assert_eq!(check(&uri.parse().expect("a uri"), TOKEN), Ok(()));
    }

    #[rstest]
    #[case::no_query("/", Refusal::NoToken)]
    #[case::another_key("/?tokens=s3cret", Refusal::NoToken)]
    #[case::an_empty_token("/?token=", Refusal::EmptyToken)]
    #[case::a_key_without_a_value("/?token", Refusal::EmptyToken)]
    #[case::the_token_twice("/?token=s3cret&token=s3cret", Refusal::TokenTwice)]
    #[case::a_wrong_token("/?token=s3creT", Refusal::WrongToken)]
    #[case::a_shorter_token("/?token=s3cre", Refusal::WrongToken)]
    #[case::a_longer_token("/?token=s3crets", Refusal::WrongToken)]
    #[case::another_path("/x?token=s3cret", Refusal::Path)]
    #[case::a_path_below_the_root("/ws/?token=s3cret", Refusal::Path)]
    fn refuses_anything_else(#[case] uri: &str, #[case] refusal: Refusal) {
        assert_eq!(check(&uri.parse().expect("a uri"), TOKEN), Err(refusal));
    }

    #[rstest]
    #[case::another_path(Refusal::Path, StatusCode::NOT_FOUND)]
    #[case::no_token(Refusal::NoToken, StatusCode::UNAUTHORIZED)]
    #[case::an_empty_token(Refusal::EmptyToken, StatusCode::UNAUTHORIZED)]
    #[case::the_token_twice(Refusal::TokenTwice, StatusCode::UNAUTHORIZED)]
    #[case::a_wrong_token(Refusal::WrongToken, StatusCode::UNAUTHORIZED)]
    #[case::too_many_connections(Refusal::Busy, StatusCode::SERVICE_UNAVAILABLE)]
    fn answers_a_refusal_with_its_status(#[case] refusal: Refusal, #[case] status: StatusCode) {
        assert_eq!(refusal.status(), status);
    }
}

mod same {
    use super::*;

    #[rstest]
    #[case::equal("s3cret", "s3cret", true)]
    #[case::both_empty("", "", true)]
    #[case::the_last_byte_differs("s3cret", "s3creT", false)]
    #[case::the_first_byte_differs("s3cret", "S3cret", false)]
    #[case::a_prefix("s3cre", "s3cret", false)]
    #[case::empty_against_a_token("", "s3cret", false)]
    fn is_true_only_for_the_same_bytes(#[case] a: &str, #[case] b: &str, #[case] expected: bool) {
        assert_eq!(same(a, b), expected);
    }
}
