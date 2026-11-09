use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use cornercase_bridge::test_util::TempDir;
use rstest::rstest;

use super::*;
use crate::machines::MachineId;
use crate::machines::route::Address;
use crate::machines::tests::{TOKEN, bridge};

const SECRET_WORD: &str = "do-not-repeat-me";

struct Folder {
    dir: TempDir,
}

impl Folder {
    fn new() -> Self {
        Self { dir: TempDir::new() }
    }

    fn file(&self) -> PathBuf {
        self.dir.path().join("machines.json")
    }

    fn store(&self) -> JsonFileStore {
        JsonFileStore::new(self.file())
    }

    fn text(&self) -> String {
        fs::read_to_string(self.file()).expect("read the file")
    }

    fn write(&self, text: &str) {
        fs::write(self.file(), text).expect("write the file");
    }
}

fn studio(id: u64, token: Option<&str>) -> Machine {
    Machine {
        id: MachineId(id),
        name: "studio".to_owned(),
        routes: vec![bridge("192.168.1.20", 8765, token), bridge("fd7a:115c:a1e0::1", 8765, token)],
        last_route: Some(Address { host: "fd7a:115c:a1e0::1".to_owned(), port: 8765 }),
    }
}

fn saved(next_id: u64) -> Saved {
    Saved { next_id, machines: (0..next_id).map(|id| studio(id, None)).collect() }
}

fn store_error<T: Debug>(result: Result<T, Error>) -> String {
    match result {
        Err(Error::Store(message)) => message,
        other => panic!("expected a store error, got {other:?}"),
    }
}

mod load {
    use super::*;

    #[test]
    fn gives_an_empty_list_when_the_file_does_not_exist() {
        let folder = Folder::new();
        assert_eq!(folder.store().load().expect("load"), Saved { next_id: 0, machines: Vec::new() });
        assert!(!folder.file().exists());
    }

    #[test]
    fn reads_back_what_was_saved() {
        let folder = Folder::new();
        folder.store().save(&saved(2)).expect("save");
        assert_eq!(folder.store().load().expect("load"), saved(2));
    }

    #[test]
    fn reads_the_routes_without_their_tokens() {
        let folder = Folder::new();
        let with_tokens = Saved { next_id: 4, machines: vec![studio(3, Some(TOKEN))] };
        folder.store().save(&with_tokens).expect("save");
        let without = Saved { next_id: 4, machines: vec![studio(3, None)] };
        assert_eq!(folder.store().load().expect("load"), without);
        assert!(!folder.text().contains(TOKEN), "{}", folder.text());
    }

    #[rstest]
    #[case::not_json(format!("{{\"version\": 1, {SECRET_WORD}"), "is not JSON")]
    #[case::cut_short(r#"{"version": 1, "next_id": 0, "machines": ["#.to_owned(), "is not JSON")]
    #[case::wrong_field(
        format!(r#"{{"version": 1, "next_id": "{SECRET_WORD}", "machines": []}}"#),
        "does not hold the expected fields"
    )]
    #[case::bad_route(
        format!(r#"{{"version": 1, "next_id": 1, "machines": [{{"id": 0, "name": "a", "routes": [{{"kind": "bridge", "url": "{SECRET_WORD}"}}], "last_route": null}}]}}"#),
        "does not hold the expected fields"
    )]
    #[case::no_version(format!(r#"{{"next_id": 0, "machines": [], "{SECRET_WORD}": 1}}"#), "does not hold the expected fields")]
    #[case::unknown_version(
        format!(r#"{{"version": 99, "next_id": 0, "machines": "{SECRET_WORD}"}}"#),
        "the file's format version 99 is unknown"
    )]
    fn refuses_a_file_it_cannot_read_without_quoting_it(#[case] text: String, #[case] reason: &str) {
        let folder = Folder::new();
        folder.write(&text);
        let message = store_error(folder.store().load());
        assert!(message.contains(reason), "{message}");
        assert!(!message.contains(SECRET_WORD), "{message}");
        assert!(!message.contains("bridge"), "{message}");
        assert_eq!(folder.text(), text);
    }

    #[test]
    fn says_why_it_cannot_read_a_folder() {
        let folder = Folder::new();
        fs::create_dir(folder.file()).expect("create a folder in the file's place");
        assert!(store_error(folder.store().load()).starts_with("cannot read the file: "));
    }
}

mod save {
    use super::*;

    #[test]
    fn writes_the_version_the_next_id_and_the_machines() {
        let folder = Folder::new();
        folder.store().save(&saved(1)).expect("save");
        let written: serde_json::Value = serde_json::from_str(&folder.text()).expect("the file is JSON");
        let machine = serde_json::to_value(studio(0, None)).expect("serialize the machine");
        assert_eq!(written, serde_json::json!({ "version": 1, "next_id": 1, "machines": [machine] }));
    }

    #[test]
    fn replaces_the_previous_file_and_leaves_no_temporary_file() {
        let folder = Folder::new();
        folder.store().save(&saved(1)).expect("save the first");
        folder.store().save(&saved(3)).expect("save the second");
        assert_eq!(folder.store().load().expect("load"), saved(3));
        let names: Vec<_> = fs::read_dir(folder.dir.path())
            .expect("list the folder")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        assert_eq!(names, ["machines.json"]);
    }

    #[test]
    fn creates_the_missing_folders() {
        let dir = TempDir::new();
        let store = JsonFileStore::new(dir.path().join("app").join("data").join("machines.json"));
        store.save(&saved(1)).expect("save");
        assert_eq!(store.load().expect("load"), saved(1));
    }

    #[test]
    fn keeps_the_previous_file_when_the_temporary_file_cannot_be_written() {
        let folder = Folder::new();
        folder.store().save(&saved(1)).expect("save the first");
        let before = folder.text();
        fs::create_dir(folder.dir.path().join(".machines.json.tmp")).expect("put a folder in the temporary's place");
        let message = store_error(folder.store().save(&saved(2)));
        assert!(message.starts_with("cannot write the temporary file: "), "{message}");
        assert_eq!(folder.text(), before);
    }

    #[test]
    fn keeps_the_previous_file_when_it_cannot_be_replaced() {
        let folder = Folder::new();
        fs::create_dir(folder.file()).expect("put a folder in the file's place");
        fs::write(folder.file().join("inside"), "kept").expect("fill the folder");
        let message = store_error(folder.store().save(&saved(2)));
        assert!(message.starts_with("cannot replace the file: "), "{message}");
        assert_eq!(fs::read_to_string(folder.file().join("inside")).expect("read the kept file"), "kept");
        assert!(!folder.dir.path().join(".machines.json.tmp").exists());
    }
}
