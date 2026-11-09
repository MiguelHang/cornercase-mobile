use rstest::rstest;

use super::*;

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

mod argv {
    use super::*;

    #[rstest]
    #[case::esc("esc")]
    #[case::tab("tab")]
    #[case::up("up")]
    #[case::down("down")]
    #[case::left("left")]
    #[case::right("right")]
    #[case::enter("enter")]
    #[case::ctrl_c("ctrl+c")]
    #[case::zero("0")]
    #[case::one("1")]
    #[case::two("2")]
    #[case::three("3")]
    #[case::four("4")]
    #[case::five("5")]
    #[case::six("6")]
    #[case::seven("7")]
    #[case::eight("8")]
    #[case::nine("9")]
    fn sends_one_named_key_to_one_pane(#[case] key: &str) {
        assert_eq!(args(12, key), Ok(words(&["keys", "--pane", "12", key])));
    }

    #[test]
    fn knows_every_key_the_app_sends_and_no_other() {
        let accepted: Vec<&str> = NAMES.into_iter().filter(|key| args(1, key).is_ok()).collect();
        assert_eq!(accepted, NAMES);
    }
}

mod refusing {
    use super::*;

    #[rstest]
    #[case::empty("")]
    #[case::a_short_option("-x")]
    #[case::a_long_option("--force")]
    #[case::the_end_of_options("--")]
    #[case::a_key_the_app_does_not_send("shift+tab")]
    #[case::another_control_key("ctrl+o")]
    #[case::a_capital("Esc")]
    #[case::two_digits("10")]
    #[case::a_space_around_it(" enter")]
    #[case::two_keys("down enter")]
    fn fails_on_a_name_outside_the_app_keys(#[case] key: &str) {
        assert_eq!(args(12, key), Err(Error::UnknownKey(key.to_owned())));
    }
}
