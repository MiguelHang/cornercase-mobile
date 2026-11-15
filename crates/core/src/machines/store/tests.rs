use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use rstest::rstest;

use super::*;
use crate::machines::route::Address;
use crate::machines::tests::{SECRET, cli};
use crate::test_util::{TempDir, V1_WITH_BRIDGES};

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
        routes: vec![cli("192.168.1.20", 8765, token), cli("fd7a:115c:a1e0::1", 8765, token)],
        last_route: Some(Address { host: "fd7a:115c:a1e0::1".to_owned(), port: 8765 }),
    }
}

fn saved(next_id: u64) -> Saved {
    Saved { next_id, machines: (0..next_id).map(|id| studio(id, None)).collect(), active: None }
}

fn whole(saved: Saved) -> Loaded {
    Loaded { saved, dropped: Vec::new() }
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
        assert_eq!(folder.store().load().expect("load"), Loaded::default());
        assert!(!folder.file().exists());
    }

    #[test]
    fn reads_back_what_was_saved() {
        let folder = Folder::new();
        folder.store().save(&saved(2)).expect("save");
        assert_eq!(folder.store().load().expect("load"), whole(saved(2)));
    }

    #[test]
    fn reads_back_the_active_machine() {
        let folder = Folder::new();
        let with_active = Saved { active: Some(MachineId(1)), ..saved(2) };
        folder.store().save(&with_active).expect("save");
        assert_eq!(folder.store().load().expect("load"), whole(with_active));
    }

    #[test]
    fn reads_a_file_written_before_the_active_machine_was_kept() {
        let folder = Folder::new();
        let machine = serde_json::to_string(&studio(0, None)).expect("serialize the machine");
        folder.write(&format!(r#"{{"version": 1, "next_id": 1, "machines": [{machine}]}}"#));
        assert_eq!(folder.store().load().expect("load"), whole(saved(1)));
    }

    #[test]
    fn reads_the_routes_without_their_secrets() {
        let folder = Folder::new();
        let with_secrets = Saved { next_id: 4, machines: vec![studio(3, Some(SECRET))], active: None };
        folder.store().save(&with_secrets).expect("save");
        let without = Saved { next_id: 4, machines: vec![studio(3, None)], active: None };
        assert_eq!(folder.store().load().expect("load"), whole(without));
        assert!(!folder.text().contains(SECRET), "{}", folder.text());
    }

    #[test]
    fn drops_the_bridge_routes_of_a_version_1_file_and_says_which() {
        let folder = Folder::new();
        folder.write(V1_WITH_BRIDGES);
        let studio = Machine {
            id: MachineId(3),
            name: "studio".to_owned(),
            routes: vec![cli("192.168.1.20", 47682, None)],
            last_route: None,
        };
        let laptop = Machine { id: MachineId(4), name: "laptop".to_owned(), routes: Vec::new(), last_route: None };
        let dropped = vec![
            (MachineId(3), Address { host: "192.168.1.20".to_owned(), port: 8765 }),
            (MachineId(4), Address { host: "10.0.0.9".to_owned(), port: 8765 }),
        ];
        let saved = Saved { next_id: 5, machines: vec![studio, laptop], active: Some(MachineId(4)) };
        assert_eq!(folder.store().load().expect("load"), Loaded { saved, dropped });
        assert_eq!(folder.text(), V1_WITH_BRIDGES);
    }

    #[rstest]
    #[case::not_json(format!("{{\"version\": 1, {SECRET_WORD}"), "is not JSON")]
    #[case::cut_short(r#"{"version": 1, "next_id": 0, "machines": ["#.to_owned(), "is not JSON")]
    #[case::wrong_field(
        format!(r#"{{"version": 1, "next_id": "{SECRET_WORD}", "machines": []}}"#),
        "does not hold the expected fields"
    )]
    #[case::bad_route(
        format!(r#"{{"version": 1, "next_id": 1, "machines": [{{"id": 0, "name": "a", "routes": [{{"kind": "cli", "url": "{SECRET_WORD}"}}], "last_route": null}}]}}"#),
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
    fn writes_the_version_the_next_id_the_machines_and_the_active_one() {
        let folder = Folder::new();
        folder.store().save(&Saved { active: Some(MachineId(0)), ..saved(1) }).expect("save");
        let written: serde_json::Value = serde_json::from_str(&folder.text()).expect("the file is JSON");
        let machine = serde_json::to_value(studio(0, None)).expect("serialize the machine");
        assert_eq!(written, serde_json::json!({ "version": 1, "next_id": 1, "machines": [machine], "active": 0 }));
    }

    #[test]
    fn writes_each_route_with_its_kind() {
        let folder = Folder::new();
        folder
            .store()
            .save(&Saved { next_id: 4, machines: vec![studio(3, Some(SECRET))], active: None })
            .expect("save");
        let written: serde_json::Value = serde_json::from_str(&folder.text()).expect("the file is JSON");
        let routes = serde_json::json!([
            { "kind": "cli", "url": "ws://192.168.1.20:8765/" },
            { "kind": "cli", "url": "ws://[fd7a:115c:a1e0::1]:8765/" },
        ]);
        assert_eq!(written["machines"][0]["routes"], routes);
    }

    #[test]
    fn replaces_the_previous_file_and_leaves_no_temporary_file() {
        let folder = Folder::new();
        folder.store().save(&saved(1)).expect("save the first");
        folder.store().save(&saved(3)).expect("save the second");
        assert_eq!(folder.store().load().expect("load"), whole(saved(3)));
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
        assert_eq!(store.load().expect("load"), whole(saved(1)));
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
