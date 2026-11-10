use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use cornercase_bridge::test_util::TempDir;
use rstest::rstest;

use super::*;
use crate::secrets::memory::MemorySecrets;

const ROUTE: &str = "route/1/100.64.0.1:8787";
const IDENTITY: &str = "identity/ed25519";
const SECRET_WORD: &str = "do-not-repeat-me";
const BYTES: [u8; 4] = [0x00, 0xff, 0x80, 0x0a];

struct Move {
    _dir: TempDir,
    path: PathBuf,
    store: MemorySecrets,
}

impl Move {
    fn new() -> Self {
        let dir = TempDir::new();
        let path = dir.path().join("route-secrets.json");
        Self { _dir: dir, path, store: MemorySecrets::default() }
    }

    fn holding(text: &str) -> Self {
        let moving = Self::new();
        fs::write(&moving.path, text).expect("write the secrets file");
        moving
    }

    fn with_two_secrets() -> Self {
        let moving = Self::new();
        moving.file().set(ROUTE, SECRET_WORD.as_bytes()).expect("set the route's token");
        moving.file().set(IDENTITY, &BYTES).expect("set the identity");
        moving
    }

    fn file(&self) -> FileSecrets {
        FileSecrets::new(self.path.clone())
    }

    fn run(&self) -> Result<Migration, Error> {
        migrate(&self.file(), &self.store)
    }

    fn temporary(&self) -> PathBuf {
        self.path.with_file_name(".route-secrets.json.tmp")
    }

    fn leave_a_temporary_copy(&self) {
        fs::write(self.temporary(), SECRET_WORD).expect("write the temporary copy");
    }

    fn folder(&self) -> &Path {
        self.path.parent().expect("the file's folder")
    }

    fn contents(&self) -> String {
        fs::read_to_string(&self.path).expect("read the secrets file")
    }

    fn assert_moved(&self) {
        assert_eq!(self.store.get(ROUTE), Ok(Some(SECRET_WORD.as_bytes().to_vec())));
        assert_eq!(self.store.get(IDENTITY), Ok(Some(BYTES.to_vec())));
        assert!(!self.path.exists());
    }
}

struct Refusing<'a> {
    store: &'a MemorySecrets,
    key: &'static str,
}

impl Secrets for Refusing<'_> {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.store.get(key)
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        if key == self.key {
            return Err(Error::Secrets("the store refuses this key".to_owned()));
        }
        self.store.set(key, value)
    }

    fn delete(&self, key: &str) -> Result<(), Error> {
        self.store.delete(key)
    }
}

fn set_mode(folder: &Path, mode: u32) {
    fs::set_permissions(folder, Permissions::from_mode(mode)).expect("set the folder's mode");
}

fn failure(result: Result<Migration, Error>) -> String {
    match result {
        Err(Error::Secrets(message)) => message,
        other => panic!("expected a secrets error, got {other:?}"),
    }
}

mod migrate {
    use super::*;

    #[test]
    fn copies_every_secret_into_the_store_and_removes_the_file() {
        let moving = Move::with_two_secrets();
        assert_eq!(moving.run(), Ok(Migration::Moved(2)));
        moving.assert_moved();
    }

    #[test]
    fn has_nothing_to_do_without_a_file() {
        let moving = Move::new();
        assert_eq!(moving.run(), Ok(Migration::NoFile));
        assert_eq!(moving.store.get(ROUTE), Ok(None));
        assert!(!moving.path.exists());
    }

    #[test]
    fn removes_a_file_without_secrets() {
        let moving = Move::holding(r#"{"version": 1, "secrets": {}}"#);
        assert_eq!(moving.run(), Ok(Migration::Moved(0)));
        assert!(!moving.path.exists());
    }

    #[test]
    fn removes_the_temporary_copy_a_write_cut_short_left() {
        let moving = Move::with_two_secrets();
        moving.leave_a_temporary_copy();
        assert_eq!(moving.run(), Ok(Migration::Moved(2)));
        moving.assert_moved();
        assert!(!moving.temporary().exists());
    }

    #[test]
    fn removes_a_temporary_copy_left_without_a_file() {
        let moving = Move::new();
        moving.leave_a_temporary_copy();
        assert_eq!(moving.run(), Ok(Migration::NoFile));
        assert!(!moving.temporary().exists());
    }

    #[test]
    fn copies_the_same_values_over_a_migration_cut_short() {
        let moving = Move::with_two_secrets();
        moving.store.set(ROUTE, b"half-written").expect("leave a copy from the run cut short");
        assert_eq!(moving.run(), Ok(Migration::Moved(2)));
        moving.assert_moved();
    }

    #[test]
    fn moves_everything_on_the_next_try_after_the_store_failed() {
        let moving = Move::with_two_secrets();
        moving.store.fail_writes(true);
        failure(moving.run());
        moving.store.fail_writes(false);
        assert_eq!(moving.run(), Ok(Migration::Moved(2)));
        moving.assert_moved();
    }
}

mod failing {
    use super::*;

    #[test]
    fn keeps_the_file_when_the_store_refuses_a_copy() {
        let moving = Move::with_two_secrets();
        let before = moving.contents();
        moving.store.fail_writes(true);
        let message = failure(moving.run());
        assert!(message.starts_with("cannot copy the secrets file into the store"), "{message}");
        assert!(!message.contains(SECRET_WORD), "{message}");
        assert_eq!(moving.contents(), before);
    }

    #[test]
    fn forgets_the_copies_already_made_when_a_later_one_is_refused() {
        let moving = Move::with_two_secrets();
        let before = moving.contents();
        let refusing = Refusing { store: &moving.store, key: ROUTE };
        failure(migrate(&moving.file(), &refusing));
        assert_eq!(moving.store.get(IDENTITY), Ok(None));
        assert_eq!(moving.contents(), before);
    }

    #[test]
    fn keeps_the_file_and_forgets_the_copies_when_the_file_cannot_be_removed() {
        let moving = Move::with_two_secrets();
        let before = moving.contents();
        set_mode(moving.folder(), 0o500);
        let result = moving.run();
        set_mode(moving.folder(), 0o700);
        assert!(failure(result).starts_with("cannot remove the secrets file"));
        assert_eq!(moving.contents(), before);
        assert_eq!(moving.store.get(IDENTITY), Ok(None));
        assert_eq!(moving.store.get(ROUTE), Ok(None));
    }

    #[rstest]
    #[case::not_json(format!("{{\"version\": 1, {SECRET_WORD}"), "is not JSON")]
    #[case::unknown_version(format!(r#"{{"version": 2, "secrets": {{"{ROUTE}": "00"}}}}"#), "version 2")]
    #[case::a_value_not_hex(
        format!(r#"{{"version": 1, "secrets": {{"{IDENTITY}": "00", "{ROUTE}": "{SECRET_WORD}"}}}}"#),
        "is not hex"
    )]
    fn keeps_a_broken_file_and_copies_nothing(#[case] text: String, #[case] expected: &str) {
        let moving = Move::holding(&text);
        let message = failure(moving.run());
        assert!(message.contains(expected), "{message}");
        assert!(!message.contains(SECRET_WORD), "{message}");
        assert_eq!(moving.contents(), text);
        assert_eq!(moving.store.get(IDENTITY), Ok(None));
    }

    #[test]
    fn fails_when_the_file_cannot_be_read() {
        let moving = Move::new();
        fs::create_dir(&moving.path).expect("put a folder in the file's place");
        assert!(failure(moving.run()).starts_with("cannot read the secrets file"));
        assert!(moving.path.is_dir());
    }
}
