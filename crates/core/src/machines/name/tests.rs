use rstest::rstest;

use super::*;

fn saved(id: u64, name: &str) -> Machine {
    Machine { id: MachineId(id), name: name.to_owned(), routes: Vec::new(), last_route: None }
}

fn studio_and_laptop() -> Vec<Machine> {
    vec![saved(0, "Studio"), saved(1, "laptop")]
}

mod checked_name {
    use super::*;

    #[rstest]
    #[case::kept_as_typed("Mac mini", "Mac mini")]
    #[case::trimmed("  Mac mini \t", "Mac mini")]
    #[case::at_the_limit(&"m".repeat(LONGEST_NAME), &"m".repeat(LONGEST_NAME))]
    #[case::counted_in_characters(&"é".repeat(LONGEST_NAME), &"é".repeat(LONGEST_NAME))]
    #[case::limit_after_trimming(&format!(" {} ", "m".repeat(LONGEST_NAME)), &"m".repeat(LONGEST_NAME))]
    fn accepts_a_free_name(#[case] name: &str, #[case] expected: &str) {
        assert_eq!(checked_name(name, &studio_and_laptop(), None), Ok(expected.to_owned()));
    }

    #[rstest]
    #[case::empty("", Error::NameEmpty)]
    #[case::only_spaces(" \t ", Error::NameEmpty)]
    #[case::one_too_many(&"m".repeat(LONGEST_NAME + 1), Error::NameTooLong)]
    #[case::taken("laptop", Error::NameTaken)]
    #[case::taken_in_other_case("STUDIO", Error::NameTaken)]
    #[case::taken_with_spaces_around("  Laptop ", Error::NameTaken)]
    fn refuses_a_name(#[case] name: &str, #[case] error: Error) {
        assert_eq!(checked_name(name, &studio_and_laptop(), None), Err(error));
    }

    #[test]
    fn lets_a_machine_keep_its_own_name_in_another_case() {
        assert_eq!(checked_name("studio", &studio_and_laptop(), Some(MachineId(0))), Ok("studio".to_owned()));
    }

    #[test]
    fn still_refuses_the_name_of_another_machine_when_renaming() {
        assert_eq!(checked_name("Laptop", &studio_and_laptop(), Some(MachineId(0))), Err(Error::NameTaken));
    }

    #[test]
    fn says_the_limit_in_its_message() {
        assert_eq!(Error::NameTooLong.to_string(), "the name is longer than 64 characters");
    }
}
