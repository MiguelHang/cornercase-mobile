use std::cell::RefCell;

use rstest::rstest;

use super::*;

const EXECUTABLES: &[&str] =
    &["/opt/cc/cornercase", "/usr/local/bin/cornercase", "/usr/bin/cornercase", "/usr/bin/cc-dev", "./dev/cornercase"];

fn fake_executable(path: &Path) -> bool {
    path.to_str().is_some_and(|path| EXECUTABLES.contains(&path))
}

fn found(option: Option<&str>, variable: Option<&str>, path: Option<&str>) -> Result<PathBuf, NotFound> {
    find(option.map(Path::new), variable.map(OsStr::new), path.map(OsStr::new), fake_executable)
}

fn not_executable(origin: Origin, path: &str) -> Result<PathBuf, NotFound> {
    Err(NotFound::NotExecutable { origin, path: PathBuf::from(path) })
}

fn not_in_path(origin: Origin, name: &str, path: Option<&str>) -> Result<PathBuf, NotFound> {
    Err(NotFound::NotInPath { origin, name: OsString::from(name), path: path.map(OsString::from) })
}

mod find {
    use super::*;

    const PATH: Option<&str> = Some("/usr/local/bin:/usr/bin");

    #[rstest]
    #[case::option_wins(
        Some("/opt/cc/cornercase"),
        Some("/usr/bin/cc-dev"),
        PATH,
        Ok(PathBuf::from("/opt/cc/cornercase"))
    )]
    #[case::relative_option_is_kept(Some("./dev/cornercase"), None, PATH, Ok(PathBuf::from("./dev/cornercase")))]
    #[case::missing_option_does_not_fall_back(
        Some("/nonexistent"),
        Some("/usr/bin/cc-dev"),
        PATH,
        not_executable(Origin::Option, "/nonexistent")
    )]
    #[case::option_name_is_not_looked_up(Some("cornercase"), None, PATH, not_executable(Origin::Option, "cornercase"))]
    #[case::empty_option_is_an_error(Some(""), Some("/usr/bin/cc-dev"), PATH, not_executable(Origin::Option, ""))]
    #[case::variable_wins_over_path(None, Some("/opt/cc/cornercase"), PATH, Ok(PathBuf::from("/opt/cc/cornercase")))]
    #[case::variable_name_is_looked_up(None, Some("cc-dev"), PATH, Ok(PathBuf::from("/usr/bin/cc-dev")))]
    #[case::variable_not_executable(
        None,
        Some("/opt/cc/cc-dev"),
        PATH,
        not_executable(Origin::Variable, "/opt/cc/cc-dev")
    )]
    #[case::variable_name_not_in_path(None, Some("cc-other"), PATH, not_in_path(Origin::Variable, "cc-other", PATH))]
    #[case::empty_variable_is_unset(None, Some(""), PATH, Ok(PathBuf::from("/usr/local/bin/cornercase")))]
    #[case::first_in_path_wins(None, None, PATH, Ok(PathBuf::from("/usr/local/bin/cornercase")))]
    #[case::later_in_path(None, None, Some("/bin:/usr/bin"), Ok(PathBuf::from("/usr/bin/cornercase")))]
    #[case::nothing_in_path(
        None,
        None,
        Some("/bin:/sbin"),
        not_in_path(Origin::Default, "cornercase", Some("/bin:/sbin"))
    )]
    #[case::empty_path(None, None, Some(""), not_in_path(Origin::Default, "cornercase", Some("")))]
    #[case::empty_entries_are_skipped(
        None,
        None,
        Some("::/bin"),
        not_in_path(Origin::Default, "cornercase", Some("::/bin"))
    )]
    #[case::unset_path(None, None, None, not_in_path(Origin::Default, "cornercase", None))]
    #[case::variable_name_with_unset_path(None, Some("cc-dev"), None, not_in_path(Origin::Variable, "cc-dev", None))]
    fn picks_the_first_defined_candidate(
        #[case] option: Option<&str>,
        #[case] variable: Option<&str>,
        #[case] path: Option<&str>,
        #[case] expected: Result<PathBuf, NotFound>,
    ) {
        assert_eq!(found(option, variable, path), expected);
    }

    #[test]
    fn looks_up_a_name_in_the_current_folder_only_through_a_slash() {
        let asked = RefCell::new(Vec::new());
        let result = find(None, Some(OsStr::new("./dev/cornercase")), PATH.map(OsStr::new), |path| {
            asked.borrow_mut().push(path.to_path_buf());
            fake_executable(path)
        });
        assert_eq!(result, Ok(PathBuf::from("./dev/cornercase")));
        assert_eq!(*asked.borrow(), vec![PathBuf::from("./dev/cornercase")]);
    }
}

mod messages {
    use super::*;

    #[rstest]
    #[case::option(
        not_executable(Origin::Option, "/nonexistent"),
        "--cornercase: /nonexistent does not exist or is not executable"
    )]
    #[case::variable(
        not_executable(Origin::Variable, "/opt/cc/cc-dev"),
        "CORNERCASE_BIN: /opt/cc/cc-dev does not exist or is not executable"
    )]
    #[case::variable_name(
        not_in_path(Origin::Variable, "cc-dev", Some("/bin")),
        "CORNERCASE_BIN: no executable `cc-dev` in the PATH (/bin)"
    )]
    #[case::default(
        not_in_path(Origin::Default, "cornercase", Some("/bin:/sbin")),
        "no executable `cornercase` in the PATH (/bin:/sbin)"
    )]
    #[case::empty_path(
        not_in_path(Origin::Default, "cornercase", Some("")),
        "no executable `cornercase` in the PATH ()"
    )]
    #[case::unset_path(not_in_path(Origin::Default, "cornercase", None), "no executable `cornercase`: PATH is not set")]
    fn name_what_was_sought_and_where(#[case] error: Result<PathBuf, NotFound>, #[case] message: &str) {
        assert_eq!(error.expect_err("an error").to_string(), message);
    }
}

mod is_executable {
    use super::*;

    #[rstest]
    #[case::a_program("/bin/sh", true)]
    #[case::a_plain_file("/etc/hosts", false)]
    #[case::a_folder("/bin", false)]
    #[case::a_missing_path("/nonexistent/cornercase", false)]
    fn needs_a_file_with_an_execute_bit(#[case] path: &str, #[case] expected: bool) {
        assert_eq!(super::is_executable(Path::new(path)), expected);
    }
}
