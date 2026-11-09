use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::cli::changes::fixtures::{EVERY, PATCH, SUMMARY};

fn summary(text: &str) -> Summary {
    serde_json::from_str(text).expect("a changes summary")
}

fn patch(text: &str) -> Patch {
    serde_json::from_str(text).expect("a changed file's patch")
}

fn file(line: Value) -> ChangedFile {
    ChangedFile::deserialize(line).expect("a changed file")
}

fn only_workspace(summary: &Summary) -> &ChangedWorkspace {
    match summary.workspaces.as_slice() {
        [workspace] => workspace,
        other => panic!("expected one workspace, got {other:?}"),
    }
}

fn is_digest(text: &str) -> bool {
    text.len() == 16 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

mod a_summary {
    use super::*;

    #[test]
    fn reads_its_header() {
        let read = summary(SUMMARY);
        assert_eq!((read.boot.as_str(), read.stream, read.revision), ("4242-1700000000", 1, 2));
        assert_eq!(
            (read.scope.as_str(), read.mode, read.state, read.error),
            ("project", Mode::Commits, State::Ready, None)
        );
    }

    #[test]
    fn reads_the_workspace_with_its_base_and_totals() {
        let read = summary(SUMMARY);
        let workspace = only_workspace(&read);
        assert_eq!((workspace.project, workspace.workspace), (1, 2));
        assert_eq!((workspace.base.as_deref(), workspace.added, workspace.removed), (Some("main"), 12, 9));
        let paths: Vec<_> = workspace.files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, vec!["Cargo.lock", "src/new.rs", "notes.txt"]);
    }

    #[test]
    fn reads_a_viewed_lockfile() {
        let expected = ChangedFile {
            path: "Cargo.lock".to_owned(),
            old_path: None,
            status: Status::Modified,
            added: 4,
            removed: 3,
            fold: Fold::Lockfile,
            viewed: true,
            staged: None,
            digest: "00c0ffee12345678".to_owned(),
        };
        assert_eq!(only_workspace(&summary(SUMMARY)).files[0], expected);
    }

    #[test]
    fn reads_a_rename_with_its_old_path() {
        let read = summary(SUMMARY);
        let renamed = &only_workspace(&read).files[1];
        assert_eq!((renamed.status, renamed.old_path.as_deref()), (Status::Renamed, Some("src/old.rs")));
        assert_eq!((renamed.fold, renamed.viewed), (Fold::Open, false));
    }

    #[test]
    fn reads_an_untracked_file() {
        let read = summary(SUMMARY);
        let untracked = &only_workspace(&read).files[2];
        assert_eq!(
            (untracked.status, untracked.old_path.as_deref(), untracked.staged),
            (Status::Untracked, None, None)
        );
    }

    #[test]
    fn reads_every_digest_as_sixteen_hex_digits() {
        let read = summary(SUMMARY);
        let digests: Vec<_> = only_workspace(&read).files.iter().map(|file| file.digest.as_str()).collect();
        assert!(digests.iter().all(|digest| is_digest(digest)), "the digests: {digests:?}");
    }
}

mod every_workspace {
    use super::*;

    #[test]
    fn reads_each_workspace_of_each_project() {
        let read = summary(EVERY);
        assert_eq!((read.scope.as_str(), read.mode, read.stream, read.revision), ("every", Mode::Uncommitted, 2, 5));
        let places: Vec<_> = read.workspaces.iter().map(|workspace| (workspace.project, workspace.workspace)).collect();
        assert_eq!(places, vec![(1, 2), (1, 3), (4, 5)]);
        assert!(read.workspaces.iter().all(|workspace| workspace.base.is_none()));
    }

    #[test]
    fn reads_how_much_of_each_file_is_staged() {
        let read = summary(EVERY);
        let staged: Vec<_> = read.workspaces.iter().map(|workspace| workspace.files[0].staged).collect();
        assert_eq!(staged, vec![Some(Staged::Yes), Some(Staged::Partly), Some(Staged::No)]);
    }
}

mod a_patch {
    use super::*;

    #[test]
    fn reads_the_file_it_belongs_to() {
        let read = patch(PATCH);
        assert_eq!((read.workspace, read.mode), (2, Mode::Uncommitted));
        assert_eq!(
            (read.file.path.as_str(), read.file.status, read.file.fold),
            ("src/a.rs", Status::Modified, Fold::Open)
        );
        assert_eq!((read.file.added, read.file.removed, read.file.viewed), (1, 1, false));
        assert!(is_digest(&read.file.digest), "the digest: {}", read.file.digest);
    }

    #[test]
    fn reads_its_hunk_header() {
        let read = patch(PATCH);
        let [hunk] = read.hunks.as_slice() else { panic!("expected one hunk, got {:?}", read.hunks) };
        assert_eq!((hunk.old_start, hunk.new_start, hunk.context.as_str()), (1, 1, "fn main() {"));
        assert_eq!(hunk.lines.len(), 4);
    }

    #[test]
    fn reads_each_line_with_its_numbers_and_emphasis() {
        let lines = patch(PATCH).hunks.remove(0).lines;
        let removed = Line {
            kind: LineKind::Removed,
            old: Some(2),
            new: None,
            text: "    say(1);".to_owned(),
            raw: None,
            emphasis: vec![[8, 9]],
        };
        assert_eq!(lines[1], removed);
        assert_eq!((lines[2].kind, lines[2].old, lines[2].new), (LineKind::Added, None, Some(2)));
        assert_eq!(lines[2].emphasis, vec![[8, 9]]);
        assert_eq!((lines[0].kind, lines[0].old, lines[0].new), (LineKind::Context, Some(1), Some(1)));
        assert_eq!(lines[0].emphasis, Vec::<[u64; 2]>::new());
    }

    #[test]
    fn keeps_the_raw_text_of_a_line_with_tabs() {
        let lines = patch(PATCH).hunks.remove(0).lines;
        assert_eq!((lines[3].text.as_str(), lines[3].raw.as_deref()), ("    done();", Some("\tdone();")));
        assert_eq!(lines[0].raw, None);
    }
}

mod leniency {
    use super::*;

    #[rstest]
    #[case::a_lockfile("lockfile", Fold::Lockfile)]
    #[case::a_binary_file("binary", Fold::Binary)]
    #[case::a_file_too_large("large", Fold::Large)]
    #[case::an_open_file("open", Fold::Open)]
    #[case::a_new_kind_of_fold("generated", Fold::Other)]
    fn reads_how_a_file_folds(#[case] fold: &str, #[case] expected: Fold) {
        assert_eq!(file(json!({ "path": "a", "fold": fold })).fold, expected);
    }

    #[rstest]
    #[case::uncommitted("uncommitted", Mode::Uncommitted)]
    #[case::commits("commits", Mode::Commits)]
    #[case::all("all", Mode::All)]
    fn reads_each_mode(#[case] mode: &str, #[case] expected: Mode) {
        assert_eq!(summary(&json!({ "mode": mode }).to_string()).mode, expected);
    }

    #[test]
    fn reads_new_values_and_extra_fields_as_other() {
        let line = json!({
            "status": "copied",
            "state": "paused",
            "scope": "machine",
            "colour": 3,
            "workspaces": [{ "workspace": 2, "tags": [], "files": [{
                "path": "a", "status": "copied", "fold": "generated", "staged": "maybe", "owner": "me"
            }] }]
        });
        let read = summary(&line.to_string());
        assert_eq!((read.state, read.scope.as_str()), (State::Other, "machine"));
        let changed = &only_workspace(&read).files[0];
        assert_eq!((changed.status, changed.fold, changed.staged), (Status::Other, Fold::Other, Some(Staged::Other)));
    }

    #[test]
    fn reads_a_new_kind_of_line_as_other() {
        let line = json!({ "hunks": [{ "lines": [{ "kind": "moved", "text": "x", "syntax": [], "marks": 1 }] }] });
        let read = patch(&line.to_string());
        assert_eq!(read.hunks[0].lines[0].kind, LineKind::Other);
    }

    #[test]
    fn reads_an_empty_line_as_a_loading_summary() {
        let read = summary("{}");
        assert_eq!((read.state, read.mode, read.workspaces.len()), (State::Loading, Mode::Uncommitted, 0));
    }

    #[test]
    fn refuses_a_digest_sent_as_a_number() {
        let error = ChangedFile::deserialize(json!({ "path": "a", "digest": 12_345_678 }))
            .expect_err("a numeric digest is not a digest");
        assert!(error.to_string().contains("string"), "the error: {error}");
    }
}
