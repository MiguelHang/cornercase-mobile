use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use cornercase_bridge::test_util::TempDir;
use cornercase_core::history::SendState;
use serde_json::{Value, json};

use super::*;
use crate::secrets::{mock_store, on_mock};
use crate::test_util::{BOOT, Ipc};

const STUDIO: MachineId = MachineId(3);
const SECRET_WORD: &str = "do-not-repeat-me";
const NO_FOLDER: Result<PathBuf, &str> = Err("no home folder");

fn state() -> AppState {
    AppState::load(NO_FOLDER, &on_mock(mock_store()), Arc::new(on_mock(mock_store())))
}

fn kept_in(dir: &TempDir) -> AppState {
    state().keeping_history(Some(dir.path().to_path_buf()))
}

fn unwritable() -> (TempDir, AppState) {
    let dir = TempDir::new();
    fs::write(dir.path().join("history"), "a file in the folder's place").expect("write the file");
    let state = kept_in(&dir);
    (dir, state)
}

fn said(text: &str) -> Vec<HistoryEntry> {
    vec![
        HistoryEntry::Gap { id: 0, seen: 1_000 },
        HistoryEntry::You {
            id: 1,
            text: text.to_owned(),
            chips: vec![format!("{text}.png")],
            state: SendState::Sent,
            reason: None,
            seen: 1_001,
        },
        HistoryEntry::Agent { id: 2, text: "Done.".to_owned(), written: None, turn_over: Some(true), seen: 1_002 },
    ]
}

fn pane_args() -> Value {
    json!({ "machine": STUDIO.0, "boot": BOOT, "pane": 7 })
}

fn save_args(entries: &[HistoryEntry]) -> Value {
    let mut args = pane_args();
    args["entries"] = serde_json::to_value(entries).expect("the entries are json");
    args
}

mod loading {
    use super::*;

    #[test]
    fn reads_back_what_was_saved() {
        let dir = TempDir::new();
        let ipc = Ipc::new(kept_in(&dir));
        assert_eq!(ipc.invoke("history_save", save_args(&said("fix the login"))), Ok(Value::Null));
        let loaded = ipc.invoke("history_load", pane_args()).expect("the history loads");
        assert_eq!(loaded, serde_json::to_value(said("fix the login")).expect("the entries are json"));
    }

    #[test]
    fn gives_nothing_for_a_pane_never_saved() {
        let dir = TempDir::new();
        let ipc = Ipc::new(kept_in(&dir));
        assert_eq!(ipc.invoke("history_load", pane_args()), Ok(json!([])));
    }

    #[test]
    fn gives_nothing_without_a_folder_and_says_so() {
        let ipc = Ipc::new(state());
        assert_eq!(ipc.invoke("history_load", pane_args()), Ok(json!([])));
        let line = load(&ipc.state(), STUDIO, BOOT, 7).expect_err("there is no folder");
        assert_eq!(line, "cornercase-mobile: the history was not read: no folder");
    }

    #[test]
    fn gives_nothing_when_it_cannot_read_and_says_only_what_failed() {
        let (_dir, state) = unwritable();
        let ipc = Ipc::new(state);
        assert_eq!(ipc.invoke("history_load", pane_args()), Ok(json!([])));
        let line = load(&ipc.state(), STUDIO, BOOT, 7).expect_err("the folder cannot be read");
        assert_eq!(line, "cornercase-mobile: the history was not read: Store");
    }
}

mod saving {
    use super::*;

    #[test]
    fn keeps_it_in_the_history_folder_of_the_cache() {
        let dir = TempDir::new();
        save(&kept_in(&dir), STUDIO, BOOT, 7, &said("fix the login")).expect("the history is saved");
        assert!(dir.path().join("history").join("3").join("7.json").is_file());
    }

    #[test]
    fn answers_ok_when_it_cannot_write() {
        let (_dir, state) = unwritable();
        let ipc = Ipc::new(state);
        assert_eq!(ipc.invoke("history_save", save_args(&said("fix the login"))), Ok(Value::Null));
    }

    #[test]
    fn logs_a_failure_without_the_text_or_the_image_names() {
        let (_dir, state) = unwritable();
        let line = save(&state, STUDIO, BOOT, 7, &said(SECRET_WORD)).expect_err("the folder cannot be written");
        assert_eq!(line, "cornercase-mobile: the history was not saved: Store");
        assert!(!line.contains(SECRET_WORD), "{line}");
    }

    #[test]
    fn answers_ok_without_a_folder_and_says_so() {
        let ipc = Ipc::new(state());
        assert_eq!(ipc.invoke("history_save", save_args(&said("fix the login"))), Ok(Value::Null));
        let line = save(&ipc.state(), STUDIO, BOOT, 7, &said(SECRET_WORD)).expect_err("there is no folder");
        assert_eq!(line, "cornercase-mobile: the history was not saved: no folder");
    }
}

mod forgetting {
    use super::*;

    #[test]
    fn removes_only_that_machine() {
        let dir = TempDir::new();
        let state = kept_in(&dir);
        save(&state, STUDIO, BOOT, 7, &said("fix the login")).expect("the history is saved");
        save(&state, MachineId(4), BOOT, 7, &said("fix the login")).expect("the history is saved");
        forget(&state, STUDIO).expect("the history is removed");
        assert_eq!(load(&state, STUDIO, BOOT, 7), Ok(vec![]));
        assert_eq!(load(&state, MachineId(4), BOOT, 7), Ok(said("fix the login")));
    }

    #[test]
    fn says_only_what_failed() {
        let (_dir, state) = unwritable();
        assert_eq!(forget(&state, STUDIO), Err("cornercase-mobile: the history was not removed: Store".to_owned()));
    }

    #[test]
    fn says_so_without_a_folder() {
        assert_eq!(
            forget(&state(), STUDIO),
            Err("cornercase-mobile: the history was not removed: no folder".to_owned())
        );
    }
}
