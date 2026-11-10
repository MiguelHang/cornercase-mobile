use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::cli::files::fixtures::{IMAGE, LISTING, SEARCH, TEXT};

fn listing(text: &str) -> Listing {
    serde_json::from_str(text).expect("a files listing")
}

fn read_file(text: &str) -> Read {
    serde_json::from_str(text).expect("a read file")
}

fn found(text: &str) -> Found {
    serde_json::from_str(text).expect("a search's results")
}

fn entry(line: Value) -> Entry {
    Entry::deserialize(line).expect("a folder's entry")
}

mod a_listing {
    use super::*;

    #[test]
    fn reads_each_folder_with_its_place_on_disk() {
        let read = listing(LISTING);
        let folders: Vec<_> = read.folders.iter().map(|folder| (folder.path.as_str(), folder.dir.as_str())).collect();
        assert_eq!(folders, vec![("", "/work/shop"), ("missing", "/work/shop/missing")]);
    }

    #[test]
    fn reads_a_changed_folder() {
        let expected = Entry {
            name: "src".to_owned(),
            kind: EntryKind::Folder,
            link: false,
            hidden: false,
            bytes: None,
            modified: Some("2023-11-14T22:13:20.000Z".to_owned()),
            status: Some(Status::Modified),
        };
        assert_eq!(listing(LISTING).folders[0].entries[0], expected);
    }

    #[test]
    fn reads_a_hidden_file_with_its_size_and_no_status() {
        let read = listing(LISTING);
        let hidden = &read.folders[0].entries[1];
        assert_eq!((hidden.name.as_str(), hidden.kind, hidden.hidden), (".env", EntryKind::File, true));
        assert_eq!((hidden.bytes, hidden.status), (Some(12), None));
    }

    #[test]
    fn reads_an_untracked_file() {
        let read = listing(LISTING);
        let untracked = &read.folders[0].entries[2];
        assert_eq!((untracked.name.as_str(), untracked.status), ("notes.txt", Some(Status::Untracked)));
    }

    #[test]
    fn reads_a_folder_that_failed_with_its_error_and_no_entries() {
        let read = listing(LISTING);
        let [whole, failed] = read.folders.as_slice() else { panic!("expected two folders, got {:?}", read.folders) };
        assert_eq!((whole.more, whole.error.as_deref()), (false, None));
        assert_eq!((failed.more, failed.error.as_deref(), failed.entries.len()), (false, Some("no such folder"), 0));
    }
}

mod a_read_file {
    use super::*;

    #[test]
    fn reads_a_text_file_with_its_language() {
        let expected = Read {
            path: "src/main.rs".to_owned(),
            file: "/work/shop/src/main.rs".to_owned(),
            kind: ReadKind::Text,
            bytes: Some(13),
            modified: Some("2026-10-26T10:00:00.000Z".to_owned()),
            language: Some("rust".to_owned()),
            text: Some("fn main() {}\n".to_owned()),
            image: None,
        };
        assert_eq!(read_file(TEXT), expected);
    }

    #[test]
    fn reads_an_image_with_its_format_size_and_data() {
        let read = read_file(IMAGE);
        assert_eq!((read.path.as_str(), read.kind, read.bytes), ("logo.png", ReadKind::Image, Some(119)));
        assert_eq!((read.language, read.text), (None, None));
        let image = read.image.expect("the image");
        assert_eq!((image.format.as_str(), image.width, image.height), ("PNG", Some(4), Some(3)));
        assert!(image.data.starts_with("iVBORw0KGgo"), "the data: {}", image.data);
    }
}

mod a_search {
    use super::*;

    #[test]
    fn reads_its_query_counts_and_mode() {
        let read = found(SEARCH);
        assert_eq!((read.query.as_str(), read.mode), ("main", SearchMode::Text));
        assert_eq!((read.matches, read.capped, read.names.len()), (1, false, 0));
    }

    #[test]
    fn reads_the_lines_found_in_each_file() {
        let read = found(SEARCH);
        let line = FoundLine { number: 1, text: "fn main() {}".to_owned(), ranges: vec![[3, 7]] };
        assert_eq!(read.files, vec![FoundFile { path: "src/main.rs".to_owned(), lines: vec![line] }]);
    }

    #[test]
    fn reads_the_names_found_with_their_matched_characters() {
        let line = json!({
            "query": "mn", "mode": "names", "capped": true, "matches": 200,
            "names": [{ "path": "src/main.rs", "indices": [4, 7] }], "files": []
        });
        let read = found(&line.to_string());
        assert_eq!((read.mode, read.matches, read.capped), (SearchMode::Names, 200, true));
        assert_eq!(read.names, vec![Name { path: "src/main.rs".to_owned(), indices: vec![4, 7] }]);
    }
}

mod leniency {
    use super::*;

    #[rstest]
    #[case::a_file("file", EntryKind::File)]
    #[case::a_folder("folder", EntryKind::Folder)]
    #[case::something_else("other", EntryKind::Other)]
    #[case::a_new_kind_of_entry("socket", EntryKind::Other)]
    fn reads_each_kind_of_entry(#[case] kind: &str, #[case] expected: EntryKind) {
        assert_eq!(entry(json!({ "name": "a", "kind": kind })).kind, expected);
    }

    #[rstest]
    #[case::text("text", ReadKind::Text)]
    #[case::an_image("image", ReadKind::Image)]
    #[case::a_binary_file("binary", ReadKind::Binary)]
    #[case::a_file_too_large("too-large", ReadKind::TooLarge)]
    #[case::a_folder("folder", ReadKind::Folder)]
    #[case::a_missing_file("missing", ReadKind::Missing)]
    #[case::a_new_kind_of_read("video", ReadKind::Other)]
    fn reads_each_kind_of_read(#[case] kind: &str, #[case] expected: ReadKind) {
        assert_eq!(read_file(&json!({ "path": "a", "kind": kind }).to_string()).kind, expected);
    }

    #[rstest]
    #[case::modified("modified", Status::Modified)]
    #[case::added("added", Status::Added)]
    #[case::deleted("deleted", Status::Deleted)]
    #[case::renamed("renamed", Status::Renamed)]
    #[case::untracked("untracked", Status::Untracked)]
    #[case::a_new_status("copied", Status::Other)]
    fn reads_each_git_status(#[case] status: &str, #[case] expected: Status) {
        assert_eq!(entry(json!({ "name": "a", "status": status })).status, Some(expected));
    }

    #[test]
    fn reads_a_new_search_mode_as_other() {
        assert_eq!(found(&json!({ "query": "a", "mode": "regex" }).to_string()).mode, SearchMode::Other);
    }

    #[test]
    fn ignores_the_stream_header_and_extra_fields() {
        let line = json!({
            "boot": "4242-1700000000", "stream": 1, "revision": 2, "workspace": 3, "owner": "me",
            "folders": [{ "path": "", "colour": 3, "entries": [{ "name": "a", "mode": 420 }] }]
        });
        let read = listing(&line.to_string());
        assert_eq!(read.folders[0].entries[0], Entry { name: "a".to_owned(), ..Entry::default() });
    }

    #[test]
    fn reads_missing_fields_as_their_defaults() {
        assert_eq!(listing("{}"), Listing::default());
        assert_eq!(read_file("{}"), Read { kind: ReadKind::Text, ..Read::default() });
        let empty = found("{}");
        assert_eq!((empty.mode, empty.matches, empty.capped, empty.names.len()), (SearchMode::Text, 0, false, 0));
        let bare = entry(json!({}));
        assert_eq!(
            (bare.kind, bare.link, bare.hidden, bare.bytes, bare.status),
            (EntryKind::File, false, false, None, None)
        );
    }

    #[test]
    fn reads_an_image_without_its_size() {
        let line = json!({ "kind": "image", "image": { "format": "WEBP", "width": null, "data": "UklG" } });
        let image = read_file(&line.to_string()).image.expect("the image");
        assert_eq!(
            (image.format.as_str(), image.width, image.height, image.data.as_str()),
            ("WEBP", None, None, "UklG")
        );
    }
}
