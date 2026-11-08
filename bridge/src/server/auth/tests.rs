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
    #[case::swapped_bytes_that_cancel_out("ab", "ba", false)]
    #[case::a_prefix("s3cre", "s3cret", false)]
    #[case::empty_against_a_token("", "s3cret", false)]
    fn is_true_only_for_the_same_bytes(#[case] a: &str, #[case] b: &str, #[case] expected: bool) {
        assert_eq!(same(a, b), expected);
    }
}

mod gate {
    use super::*;

    const RIGHT: &str = "/?token=s3cret";

    struct Opened {
        answer: Result<(), StatusCode>,
        taken: bool,
        left: usize,
    }

    fn open(places: usize, uri: &str) -> Opened {
        let permits = Arc::new(Semaphore::new(places));
        let mut permit = None;
        let peer = "192.0.2.1:5000".parse().expect("a peer");
        let gate = Gate { token: TOKEN, peer, permits: &permits, permit: &mut permit };
        let request = Request::builder().uri(uri).body(()).expect("a request");
        let answer = gate.on_request(&request, Response::new(())).map(|_| ()).map_err(|response| response.status());
        Opened { answer, taken: permit.is_some(), left: permits.available_permits() }
    }

    #[rstest]
    #[case::full_without_a_token(0, "/", StatusCode::UNAUTHORIZED)]
    #[case::full_with_a_wrong_token(0, "/?token=wrong", StatusCode::UNAUTHORIZED)]
    #[case::full_on_another_path(0, "/x?token=s3cret", StatusCode::NOT_FOUND)]
    #[case::full_with_the_token(0, RIGHT, StatusCode::SERVICE_UNAVAILABLE)]
    #[case::free_without_a_token(1, "/", StatusCode::UNAUTHORIZED)]
    fn refuses_without_taking_a_place(#[case] places: usize, #[case] uri: &str, #[case] status: StatusCode) {
        let opened = open(places, uri);
        assert_eq!(opened.answer, Err(status));
        assert!(!opened.taken, "no place is held");
        assert_eq!(opened.left, places);
    }

    #[test]
    fn takes_a_place_for_the_right_token() {
        let opened = open(2, RIGHT);
        assert_eq!(opened.answer, Ok(()));
        assert!(opened.taken, "the place is held");
        assert_eq!(opened.left, 1);
    }
}
