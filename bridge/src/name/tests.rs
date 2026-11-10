use rstest::rstest;

use super::*;
use crate::test_util::{TempDir, write_executable};

mod machine_name {
    use super::*;

    #[rstest]
    #[case::the_computer_name(Some("MacBook Pro de Miguel\n"), "miguels-mbp.local", Some("MacBook Pro de Miguel"))]
    #[case::the_hostname_without_a_computer_name(None, "miguels-mbp.local", Some("miguels-mbp"))]
    #[case::the_hostname_when_the_computer_name_is_blank(Some(" \n"), "miguels-mbp.local", Some("miguels-mbp"))]
    #[case::a_hostname_without_local(None, "studio.lan\n", Some("studio.lan"))]
    #[case::local_only_at_the_end(None, "local.example", Some("local.example"))]
    #[case::nothing_from_an_empty_hostname(None, "", None)]
    #[case::nothing_from_local_alone(None, ".local", None)]
    fn prefers_the_computer_name_then_the_hostname_without_local(
        #[case] computer_name: Option<&str>,
        #[case] hostname: &str,
        #[case] wanted: Option<&str>,
    ) {
        assert_eq!(machine_name(computer_name.map(str::to_owned), hostname).as_deref(), wanted);
    }
}

mod computer_name {
    use super::*;

    #[rstest]
    #[case::what_it_prints("printf 'Studio\\n'", Some("Studio\n"))]
    #[case::nothing_when_it_fails("printf 'Studio'; exit 1", None)]
    fn reads_what_scutil_prints(#[case] script: &str, #[case] wanted: Option<&str>) {
        let dir = TempDir::new();
        let scutil = dir.path().join("scutil");
        write_executable(&scutil, &format!("#!/bin/sh\n{script}\n"));
        assert_eq!(computer_name(&scutil).as_deref(), wanted);
    }

    #[test]
    fn nothing_without_scutil() {
        let dir = TempDir::new();
        assert_eq!(computer_name(&dir.path().join("scutil")), None);
    }
}

mod read {
    use super::*;

    #[test]
    fn gives_a_name_with_nothing_around_it() {
        let name = read().expect("the machine has at least a hostname");
        assert_eq!(name.trim(), name);
        assert!(!name.is_empty() && !name.ends_with(LOCAL), "{name:?}");
        let hostname = rustix::system::uname().nodename().to_string_lossy().into_owned();
        assert_eq!(Some(name), machine_name(computer_name(Path::new(SCUTIL)), &hostname));
    }
}
