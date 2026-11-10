use std::fs;
use std::path::PathBuf;

use cornercase_bridge::test_util::TempDir;

use super::*;
use crate::history::SendState;

const BOOT: &str = "1731250000-4242";
const OTHER_BOOT: &str = "1731260000-777";
const STUDIO: MachineId = MachineId(3);
const LAPTOP: MachineId = MachineId(4);
const SECRET_WORD: &str = "do-not-repeat-me";

struct Folder {
    dir: TempDir,
}

impl Folder {
    fn new() -> Self {
        Self { dir: TempDir::new() }
    }

    fn store(&self) -> JsonFolderStore {
        JsonFolderStore::new(self.dir.path().join("history"))
    }

    fn machine(&self, machine: MachineId) -> PathBuf {
        self.dir.path().join("history").join(machine.0.to_string())
    }

    fn file(&self, machine: MachineId, name: &str) -> PathBuf {
        self.machine(machine).join(name)
    }

    fn names(&self, machine: MachineId) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(self.machine(machine))
            .expect("list the folder")
            .map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn index(&self, machine: MachineId) -> serde_json::Value {
        let text = fs::read(self.file(machine, INDEX)).expect("read the index");
        serde_json::from_slice(&text).expect("the index is JSON")
    }
}

fn said(text: &str) -> Vec<HistoryEntry> {
    vec![
        HistoryEntry::Gap { id: 0, seen: 1_000 },
        HistoryEntry::You {
            id: 1,
            text: text.to_owned(),
            chips: vec!["screenshot.png".to_owned()],
            state: SendState::Sent,
            reason: None,
            seen: 1_001,
        },
        HistoryEntry::Agent { id: 2, text: "Done.".to_owned(), written: None, turn_over: Some(true), seen: 1_002 },
    ]
}

fn store_error<T: std::fmt::Debug>(result: Result<T, Error>) -> String {
    match result {
        Err(Error::Store(message)) => message,
        other => panic!("expected a store error, got {other:?}"),
    }
}

mod load {
    use super::*;

    #[test]
    fn gives_nothing_for_a_pane_never_saved() {
        let folder = Folder::new();
        assert_eq!(folder.store().load(STUDIO, BOOT, 7).expect("load"), []);
        assert!(!folder.machine(STUDIO).exists());
    }

    #[test]
    fn reads_back_what_another_store_saved() {
        let folder = Folder::new();
        folder.store().save(STUDIO, BOOT, 7, &said("fix the login")).expect("save");
        assert_eq!(folder.store().load(STUDIO, BOOT, 7).expect("load"), said("fix the login"));
    }

    #[test]
    fn keeps_each_machine_and_pane_apart() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("studio seven")).expect("save");
        store.save(STUDIO, BOOT, 8, &said("studio eight")).expect("save");
        store.save(LAPTOP, OTHER_BOOT, 7, &said("laptop seven")).expect("save");
        assert_eq!(store.load(STUDIO, BOOT, 7).expect("load"), said("studio seven"));
        assert_eq!(store.load(STUDIO, BOOT, 8).expect("load"), said("studio eight"));
        assert_eq!(store.load(LAPTOP, OTHER_BOOT, 7).expect("load"), said("laptop seven"));
    }

    #[test]
    fn empties_the_machine_when_its_server_started_again() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("old")).expect("save");
        store.save(LAPTOP, BOOT, 7, &said("other machine")).expect("save");
        assert_eq!(store.load(STUDIO, OTHER_BOOT, 7).expect("load"), []);
        assert!(!folder.machine(STUDIO).exists());
        assert_eq!(store.load(STUDIO, BOOT, 7).expect("load"), []);
        assert_eq!(store.load(LAPTOP, BOOT, 7).expect("load"), said("other machine"));
    }

    #[test]
    fn reads_a_pane_it_cannot_parse_as_empty_and_the_next_save_replaces_it() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("kept")).expect("save");
        fs::write(folder.file(STUDIO, "7.json"), format!("{{\"version\": 1, {SECRET_WORD}")).expect("break the file");
        assert_eq!(store.load(STUDIO, BOOT, 7).expect("load"), []);
        store.save(STUDIO, BOOT, 7, &said("again")).expect("save");
        assert_eq!(store.load(STUDIO, BOOT, 7).expect("load"), said("again"));
    }

    #[test]
    fn reads_a_pane_of_an_unknown_version_as_empty() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("kept")).expect("save");
        fs::write(folder.file(STUDIO, "7.json"), format!(r#"{{"version": 2, "boot": "{BOOT}", "entries": []}}"#))
            .expect("write a newer file");
        assert_eq!(store.load(STUDIO, BOOT, 7).expect("load"), []);
    }

    #[test]
    fn empties_the_machine_when_its_index_cannot_be_read() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("kept")).expect("save");
        fs::write(folder.file(STUDIO, INDEX), "not json").expect("break the index");
        assert_eq!(store.load(STUDIO, BOOT, 7).expect("load"), []);
        assert!(!folder.machine(STUDIO).exists());
    }

    #[test]
    fn says_why_it_cannot_read_a_pane() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("kept")).expect("save");
        fs::remove_file(folder.file(STUDIO, "7.json")).expect("remove the file");
        fs::create_dir(folder.file(STUDIO, "7.json")).expect("put a folder in the file's place");
        assert!(store_error(store.load(STUDIO, BOOT, 7)).starts_with("cannot read the history: "));
    }
}

mod save {
    use super::*;

    #[test]
    fn writes_the_pane_and_an_index_of_the_boot_and_the_panes_newest_first() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("a")).expect("save");
        store.save(STUDIO, BOOT, 8, &said("b")).expect("save");
        store.save(STUDIO, BOOT, 7, &said("c")).expect("save");
        assert_eq!(folder.index(STUDIO), serde_json::json!({"version": 1, "boot": BOOT, "panes": [7, 8]}));
        assert_eq!(folder.names(STUDIO), ["7.json", "8.json", INDEX]);
    }

    #[test]
    fn drops_the_pane_written_longest_ago_past_fifty() {
        let folder = Folder::new();
        let store = folder.store();
        (0..50).for_each(|pane| store.save(STUDIO, BOOT, pane, &said("turn")).expect("save"));
        store.save(STUDIO, BOOT, 0, &said("again")).expect("save");
        store.save(STUDIO, BOOT, 50, &said("new")).expect("save");
        assert_eq!(store.load(STUDIO, BOOT, 1).expect("load"), []);
        assert!(!folder.file(STUDIO, "1.json").exists());
        assert_eq!(store.load(STUDIO, BOOT, 0).expect("load"), said("again"));
        assert_eq!(store.load(STUDIO, BOOT, 50).expect("load"), said("new"));
        assert_eq!(folder.names(STUDIO).len(), 51);
    }

    #[test]
    fn empties_the_machine_before_writing_for_a_new_server() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("old")).expect("save");
        store.save(LAPTOP, BOOT, 7, &said("other machine")).expect("save");
        store.save(STUDIO, OTHER_BOOT, 8, &said("new")).expect("save");
        assert_eq!(folder.names(STUDIO), ["8.json", INDEX]);
        assert_eq!(store.load(STUDIO, OTHER_BOOT, 8).expect("load"), said("new"));
        assert_eq!(store.load(LAPTOP, BOOT, 7).expect("load"), said("other machine"));
    }

    #[test]
    fn leaves_no_temporary_file() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("first")).expect("save");
        store.save(STUDIO, BOOT, 7, &said("second")).expect("save");
        assert_eq!(folder.names(STUDIO), ["7.json", INDEX]);
    }

    #[test]
    fn keeps_the_previous_pane_when_the_temporary_file_cannot_be_written() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("first")).expect("save");
        fs::create_dir(folder.file(STUDIO, ".7.json.tmp")).expect("put a folder in the temporary's place");
        let message = store_error(store.save(STUDIO, BOOT, 7, &said(SECRET_WORD)));
        assert!(message.starts_with("cannot write the temporary file: "), "{message}");
        assert!(!message.contains(SECRET_WORD), "{message}");
        assert_eq!(store.load(STUDIO, BOOT, 7).expect("load"), said("first"));
    }
}

mod forget {
    use super::*;

    #[test]
    fn removes_the_machine_and_keeps_the_others() {
        let folder = Folder::new();
        let store = folder.store();
        store.save(STUDIO, BOOT, 7, &said("studio")).expect("save");
        store.save(LAPTOP, BOOT, 7, &said("laptop")).expect("save");
        store.forget(STUDIO).expect("forget");
        assert!(!folder.machine(STUDIO).exists());
        assert_eq!(store.load(LAPTOP, BOOT, 7).expect("load"), said("laptop"));
    }

    #[test]
    fn forgets_a_machine_with_no_history() {
        let folder = Folder::new();
        folder.store().forget(STUDIO).expect("forget");
        assert!(!folder.machine(STUDIO).exists());
    }

    #[test]
    fn says_why_it_cannot_remove_the_machine() {
        let folder = Folder::new();
        fs::create_dir_all(folder.dir.path().join("history")).expect("create the folder");
        fs::write(folder.machine(STUDIO), "a file in the folder's place").expect("write the file");
        assert!(store_error(folder.store().forget(STUDIO)).starts_with("cannot remove the history: "));
    }
}
