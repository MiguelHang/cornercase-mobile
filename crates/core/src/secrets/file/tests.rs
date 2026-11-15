use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use rstest::rstest;

use super::*;
use crate::test_util::TempDir;

const KEY: &str = "route/1/100.64.0.1:8787";
const SECRET_WORD: &str = "do-not-repeat-me";

struct Folder {
    dir: TempDir,
    file: PathBuf,
}

impl Folder {
    fn new() -> Self {
        let dir = TempDir::new();
        let file = dir.path().join("route-secrets.json");
        Self { dir, file }
    }

    fn holding(text: &str) -> Self {
        let folder = Self::new();
        fs::write(&folder.file, text).expect("write the secrets file");
        folder
    }

    fn secrets(&self) -> FileSecrets {
        FileSecrets::new(self.file.clone())
    }

    fn with(&self, key: &str, value: &[u8]) -> FileSecrets {
        let secrets = self.secrets();
        secrets.set(key, value).expect("set the secret");
        secrets
    }

    fn contents(&self) -> String {
        fs::read_to_string(&self.file).expect("read the secrets file")
    }
}

fn secrets_error<T: Debug>(result: Result<T, Error>) -> String {
    match result {
        Err(Error::Secrets(message)) => message,
        other => panic!("expected a secrets error, got {other:?}"),
    }
}

mod get {
    use super::*;

    #[rstest]
    #[case::text(b"a-token".to_vec())]
    #[case::bytes_that_are_not_utf8(vec![0x00, 0xff, 0xfe, 0x80, 0x0a])]
    #[case::nothing(Vec::new())]
    fn reads_what_was_set(#[case] value: Vec<u8>) {
        let folder = Folder::new();
        let secrets = folder.with(KEY, &value);
        assert_eq!(secrets.get(KEY), Ok(Some(value)));
    }

    #[test]
    fn finds_nothing_under_an_unknown_key() {
        let folder = Folder::new();
        let secrets = folder.with(KEY, b"one");
        assert_eq!(secrets.get("route/1/100.64.0.2:8787"), Ok(None));
    }

    #[test]
    fn finds_nothing_when_the_file_does_not_exist() {
        let folder = Folder::new();
        assert_eq!(folder.secrets().get(KEY), Ok(None));
        assert!(!folder.file.exists());
    }

    #[test]
    fn reads_what_another_instance_set_on_the_same_file() {
        let folder = Folder::new();
        folder.with(KEY, b"kept");
        assert_eq!(folder.secrets().get(KEY), Ok(Some(b"kept".to_vec())));
    }

    #[test]
    fn fails_when_the_file_cannot_be_read() {
        let folder = Folder::new();
        fs::create_dir(&folder.file).expect("put a folder in the file's place");
        assert!(secrets_error(folder.secrets().get(KEY)).starts_with("cannot read the secrets file"));
    }
}

mod set {
    use super::*;

    #[test]
    fn writes_a_versioned_file_with_the_value_in_hex() {
        let folder = Folder::new();
        folder.with(KEY, &[0x00, 0xab, 0x7f]);
        let written: Value = serde_json::from_str(&folder.contents()).expect("parse the file");
        assert_eq!(written, serde_json::json!({ "version": 1, "secrets": { KEY: "00ab7f" } }));
    }

    #[test]
    fn replaces_the_value_under_the_same_key() {
        let folder = Folder::new();
        let secrets = folder.with(KEY, b"one");
        secrets.set(KEY, b"two").expect("replace the secret");
        assert_eq!(secrets.get(KEY), Ok(Some(b"two".to_vec())));
    }

    #[test]
    fn keeps_the_other_keys() {
        let folder = Folder::new();
        let secrets = folder.with(KEY, b"one");
        secrets.set("identity/ed25519", b"two").expect("set another secret");
        assert_eq!(secrets.get(KEY), Ok(Some(b"one".to_vec())));
        assert_eq!(secrets.get("identity/ed25519"), Ok(Some(b"two".to_vec())));
    }

    #[test]
    fn fails_when_the_path_is_a_folder() {
        let folder = Folder::new();
        fs::create_dir(&folder.file).expect("put a folder in the file's place");
        assert!(matches!(folder.secrets().set(KEY, b"one"), Err(Error::Secrets(_))));
    }

    #[test]
    fn fails_and_keeps_the_file_when_the_temporary_file_cannot_be_written() {
        let folder = Folder::new();
        let secrets = folder.with(KEY, b"one");
        let before = folder.contents();
        fs::create_dir(folder.dir.path().join(".route-secrets.json.tmp")).expect("block the temporary file");
        let message = secrets_error(secrets.set(KEY, b"two"));
        assert!(message.starts_with("cannot write the temporary file"), "{message}");
        assert_eq!(folder.contents(), before);
    }

    #[test]
    fn keeps_every_key_set_from_several_threads_at_once() {
        let folder = Folder::new();
        let secrets = folder.secrets();
        let keys: Vec<String> = (0..8).map(|id| format!("route/{id}/100.64.0.1:8787")).collect();
        std::thread::scope(|scope| {
            for key in &keys {
                let secrets = &secrets;
                scope.spawn(move || secrets.set(key, key.as_bytes()).expect("set the secret"));
            }
        });
        for key in &keys {
            assert_eq!(folder.secrets().get(key), Ok(Some(key.as_bytes().to_vec())));
        }
    }
}

mod delete {
    use super::*;

    #[test]
    fn removes_the_value_and_keeps_the_others() {
        let folder = Folder::new();
        let secrets = folder.with(KEY, b"one");
        secrets.set("identity/ed25519", b"two").expect("set another secret");
        secrets.delete(KEY).expect("delete the secret");
        assert_eq!(secrets.get(KEY), Ok(None));
        assert_eq!(folder.secrets().get("identity/ed25519"), Ok(Some(b"two".to_vec())));
    }

    #[test]
    fn accepts_an_unknown_key_without_writing() {
        let folder = Folder::new();
        let secrets = folder.with(KEY, b"one");
        let before = folder.contents();
        assert_eq!(secrets.delete("route/1/100.64.0.2:8787"), Ok(()));
        assert_eq!(folder.contents(), before);
    }

    #[test]
    fn accepts_a_missing_file_without_creating_it() {
        let folder = Folder::new();
        assert_eq!(folder.secrets().delete(KEY), Ok(()));
        assert!(!folder.file.exists());
    }
}

mod broken_file {
    use super::*;

    #[rstest]
    #[case::not_json(format!("{{\"version\": 1, {SECRET_WORD}"), "is not JSON")]
    #[case::cut_short(r#"{"version": 1, "secrets": {"#.to_owned(), "is not JSON")]
    #[case::secrets_not_a_map(format!(r#"{{"version": 1, "secrets": ["{SECRET_WORD}"]}}"#), "expected fields")]
    #[case::value_not_text(format!(r#"{{"version": 1, "secrets": {{"{KEY}": [1]}}}}"#), "expected fields")]
    #[case::no_secrets(r#"{"version": 1}"#.to_owned(), "expected fields")]
    #[case::no_version(format!(r#"{{"secrets": {{"{KEY}": "{SECRET_WORD}"}}}}"#), "expected fields")]
    #[case::unknown_version(format!(r#"{{"version": 2, "secrets": {{"{KEY}": "{SECRET_WORD}"}}}}"#), "version 2")]
    fn fails_without_repeating_its_contents_or_replacing_it(#[case] text: String, #[case] expected: &str) {
        let folder = Folder::holding(&text);
        let secrets = folder.secrets();
        for message in [
            secrets_error(secrets.get(KEY)),
            secrets_error(secrets.set(KEY, b"new")),
            secrets_error(secrets.delete(KEY)),
        ] {
            assert!(message.contains(expected), "{message}");
            assert!(!message.contains(SECRET_WORD), "{message}");
        }
        assert_eq!(folder.contents(), text);
    }

    #[rstest]
    #[case::not_hex_digits(SECRET_WORD)]
    #[case::odd_length("abc")]
    #[case::a_sign("+f")]
    #[case::not_ascii("éé")]
    fn fails_to_read_a_value_that_is_not_hex(#[case] value: &str) {
        let folder = Folder::holding(&format!(r#"{{"version": 1, "secrets": {{"{KEY}": "{value}"}}}}"#));
        let message = secrets_error(folder.secrets().get(KEY));
        assert_eq!(message, "a secret in the secrets file is not hex");
    }

    #[test]
    fn keeps_a_value_that_is_not_hex_while_setting_another() {
        let folder = Folder::holding(&format!(r#"{{"version": 1, "secrets": {{"{KEY}": "{SECRET_WORD}"}}}}"#));
        let secrets = folder.secrets();
        secrets.set("identity/ed25519", b"two").expect("set another secret");
        assert!(matches!(secrets.get(KEY), Err(Error::Secrets(_))));
        assert_eq!(secrets.get("identity/ed25519"), Ok(Some(b"two".to_vec())));
    }
}
