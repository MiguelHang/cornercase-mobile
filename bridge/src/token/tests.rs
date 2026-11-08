use std::fs::Permissions;
use std::thread;

use rstest::rstest;

use super::*;
use crate::test_util::TempDir;

const TOKEN_LENGTH: usize = 43;

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).expect("read the metadata").permissions().mode() & 0o777
}

fn token_in(dir: &TempDir) -> PathBuf {
    dir.path().join(FOLDER).join(FILE)
}

fn written_by_hand(dir: &TempDir, contents: &[u8], mode: u32) -> PathBuf {
    let path = token_in(dir);
    fs::create_dir_all(path.parent().expect("a folder")).expect("create the folder");
    fs::write(&path, contents).expect("write the token");
    fs::set_permissions(&path, Permissions::from_mode(mode)).expect("set the mode");
    path
}

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .expect("read the folder")
        .map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn assert_new_token(token: &str, path: &Path) {
    assert_eq!(token.len(), TOKEN_LENGTH);
    assert!(valid(token), "{token} is URL-safe");
    assert_eq!(fs::read_to_string(path).expect("read the token"), format!("{token}\n"));
    assert_eq!(mode_of(path), FILE_MODE);
    assert_eq!(names_in(path.parent().expect("a folder")), [FILE]);
}

mod path {
    use super::*;

    #[rstest]
    #[case::config_home_wins(Some("/xdg"), Some("/home/me"), "/xdg/cornercase-bridge/token")]
    #[case::config_home_without_home(Some("/xdg"), None, "/xdg/cornercase-bridge/token")]
    #[case::relative_config_home_is_ignored(Some("xdg"), Some("/home/me"), "/home/me/.config/cornercase-bridge/token")]
    #[case::empty_config_home_is_ignored(Some(""), Some("/home/me"), "/home/me/.config/cornercase-bridge/token")]
    #[case::home_without_config_home(None, Some("/home/me"), "/home/me/.config/cornercase-bridge/token")]
    fn is_under_the_config_folder(#[case] config_home: Option<&str>, #[case] home: Option<&str>, #[case] wanted: &str) {
        assert_eq!(path(config_home.map(OsStr::new), home.map(OsStr::new)), Ok(PathBuf::from(wanted)));
    }

    #[rstest]
    #[case::no_home(None, None)]
    #[case::empty_home(None, Some(""))]
    #[case::relative_home(None, Some("home/me"))]
    #[case::relative_config_home_and_no_home(Some("xdg"), None)]
    fn needs_an_absolute_home(#[case] config_home: Option<&str>, #[case] home: Option<&str>) {
        assert_eq!(path(config_home.map(OsStr::new), home.map(OsStr::new)), Err(NoHome));
    }

    #[test]
    fn says_why_there_is_no_place() {
        assert_eq!(NoHome.to_string(), "HOME is not set to an absolute path, so there is no place for the token");
    }
}

mod load_or_create {
    use super::*;

    #[test]
    fn creates_a_token_only_its_owner_reads() {
        let dir = TempDir::new();
        let path = token_in(&dir);
        let token = load_or_create(&path).expect("create the token");
        assert_new_token(&token, &path);
        assert_eq!(mode_of(path.parent().expect("a folder")), FOLDER_MODE);
    }

    #[test]
    fn gives_the_same_token_when_read_again() {
        let dir = TempDir::new();
        let path = token_in(&dir);
        let token = load_or_create(&path).expect("create the token");
        assert_eq!(load_or_create(&path).expect("read the token"), token);
    }

    #[test]
    fn gives_one_token_to_bridges_that_start_together() {
        let dir = TempDir::new();
        let path = token_in(&dir);
        let tokens: Vec<String> = thread::scope(|scope| {
            let starts: Vec<_> = (0..8).map(|_| scope.spawn(|| load_or_create(&path))).collect();
            starts.into_iter().map(|start| start.join().expect("no panic").expect("a token")).collect()
        });
        assert!(tokens.iter().all(|token| *token == tokens[0]), "{tokens:?}");
        assert_new_token(&tokens[0], &path);
    }

    #[test]
    fn keeps_a_token_written_while_it_was_making_one() {
        let dir = TempDir::new();
        let path = written_by_hand(&dir, b"written-first\n", 0o600);
        assert_eq!(create(&path).expect("read the token"), "written-first");
        assert_eq!(names_in(path.parent().expect("a folder")), [FILE]);
    }

    #[rstest]
    #[case::final_newline(b"a-Z_09\n", 0o600, "a-Z_09")]
    #[case::no_final_newline(b"a-Z_09", 0o600, "a-Z_09")]
    #[case::read_only(b"a-Z_09\n", 0o400, "a-Z_09")]
    fn reads_a_token_written_by_hand(#[case] contents: &[u8], #[case] mode: u32, #[case] wanted: &str) {
        let dir = TempDir::new();
        let path = written_by_hand(&dir, contents, mode);
        assert_eq!(load_or_create(&path).expect("read the token"), wanted);
    }

    #[rstest]
    #[case::empty(b"")]
    #[case::only_a_newline(b"\n")]
    #[case::two_newlines(b"token\n\n")]
    #[case::a_carriage_return(b"token\r\n")]
    #[case::a_space(b"to ken")]
    #[case::standard_alphabet(b"to+k/en")]
    #[case::padding(b"token==")]
    #[case::not_utf8(b"tok\xffen")]
    fn refuses_what_is_not_a_url_safe_token(#[case] contents: &[u8]) {
        let dir = TempDir::new();
        let path = written_by_hand(&dir, contents, 0o600);
        let error = load_or_create(&path).expect_err("refuse the token");
        assert_eq!(
            error.to_string(),
            format!("{} does not hold a token; run with --new-token to make a new one", path.display())
        );
    }

    #[rstest]
    #[case::group_reads(0o640)]
    #[case::others_read(0o604)]
    #[case::group_writes(0o620)]
    #[case::everyone(0o666)]
    fn refuses_a_token_other_users_reach(#[case] mode: u32) {
        let dir = TempDir::new();
        let path = written_by_hand(&dir, b"token\n", mode);
        let error = load_or_create(&path).expect_err("refuse the token");
        assert_eq!(
            error.to_string(),
            format!(
                "{} is open to other users (mode {mode:03o}); run with --new-token to make a new one",
                path.display()
            )
        );
    }

    #[test]
    fn says_where_it_could_not_read() {
        let dir = TempDir::new();
        let path = token_in(&dir);
        fs::create_dir_all(&path).expect("a folder where the token goes");
        fs::set_permissions(&path, Permissions::from_mode(0o700)).expect("set the mode");
        let error = load_or_create(&path).expect_err("no token");
        assert!(matches!(&error, Error::Io { path: at, .. } if at == &path), "{error:?}");
    }
}

mod rotate {
    use super::*;

    #[test]
    fn replaces_the_token() {
        let dir = TempDir::new();
        let path = token_in(&dir);
        let old = load_or_create(&path).expect("create the token");
        let new = rotate(&path).expect("rotate the token");
        assert_ne!(new, old);
        assert_new_token(&new, &path);
        assert_eq!(load_or_create(&path).expect("read the token"), new);
    }

    #[test]
    fn makes_the_first_token() {
        let dir = TempDir::new();
        let path = token_in(&dir);
        let token = rotate(&path).expect("make the token");
        assert_new_token(&token, &path);
        assert_eq!(mode_of(path.parent().expect("a folder")), FOLDER_MODE);
    }

    #[test]
    fn replaces_a_token_other_users_reach() {
        let dir = TempDir::new();
        let path = written_by_hand(&dir, b"token\n", 0o644);
        let token = rotate(&path).expect("rotate the token");
        assert_new_token(&token, &path);
    }

    #[test]
    fn says_where_it_could_not_make_the_folder() {
        let dir = TempDir::new();
        fs::write(dir.path().join(FOLDER), "").expect("write a file where the folder goes");
        let path = token_in(&dir);
        let error = rotate(&path).expect_err("no folder");
        assert!(matches!(&error, Error::Io { path: at, .. } if at == &dir.path().join(FOLDER)), "{error:?}");
        assert!(error.to_string().starts_with(&format!("{}: ", dir.path().join(FOLDER).display())), "{error}");
    }

    #[test]
    fn leaves_nothing_behind_when_it_cannot_replace() {
        let dir = TempDir::new();
        let path = token_in(&dir);
        fs::create_dir_all(path.join("inside")).expect("a full folder where the token goes");
        let error = rotate(&path).expect_err("no rotation");
        assert!(matches!(&error, Error::Io { path: at, .. } if at == &path), "{error:?}");
        assert_eq!(names_in(path.parent().expect("a folder")), [FILE]);
    }
}

mod error {
    use super::*;

    #[test]
    fn says_why_there_are_no_random_bytes() {
        assert!(
            Error::Random(getrandom::Error::UNSUPPORTED).to_string().starts_with("no random bytes for the token: "),
            "the reason comes first"
        );
    }
}
