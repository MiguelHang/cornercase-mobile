use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use cornercase_bridge::test_util::TempDir;
use rstest::rstest;

use super::*;

struct Folder {
    dir: TempDir,
}

impl Folder {
    fn new() -> Self {
        Self { dir: TempDir::new() }
    }

    fn file(&self) -> PathBuf {
        self.dir.path().join("data.json")
    }

    fn temporary(&self) -> PathBuf {
        self.dir.path().join(".data.json.tmp")
    }

    fn names(&self) -> Vec<OsString> {
        let entries = fs::read_dir(self.dir.path()).expect("list the folder");
        entries.map(|entry| entry.expect("an entry").file_name()).collect()
    }
}

mod write_atomically {
    use super::*;

    #[test]
    fn writes_the_bytes_and_leaves_no_temporary_file() {
        let folder = Folder::new();
        write_atomically(&folder.file(), b"first").expect("write");
        assert_eq!(fs::read(folder.file()).expect("read the file"), b"first");
        assert_eq!(folder.names(), ["data.json"]);
    }

    #[test]
    fn replaces_the_previous_file() {
        let folder = Folder::new();
        write_atomically(&folder.file(), b"a longer first text").expect("write the first");
        write_atomically(&folder.file(), b"second").expect("write the second");
        assert_eq!(fs::read(folder.file()).expect("read the file"), b"second");
        assert_eq!(folder.names(), ["data.json"]);
    }

    #[test]
    fn leaves_the_file_readable_only_by_its_owner() {
        let folder = Folder::new();
        write_atomically(&folder.file(), b"secret").expect("write");
        let mode = fs::metadata(folder.file()).expect("the file's metadata").permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "{mode:o}");
    }

    #[test]
    fn creates_the_missing_folders() {
        let dir = TempDir::new();
        let file = dir.path().join("app").join("data").join("data.json");
        write_atomically(&file, b"kept").expect("write");
        assert_eq!(fs::read(file).expect("read the file"), b"kept");
    }

    #[test]
    fn says_it_cannot_create_the_folder_when_a_file_is_in_its_place() {
        let folder = Folder::new();
        fs::write(folder.dir.path().join("app"), "a file").expect("put a file in the folder's place");
        let error = write_atomically(&folder.dir.path().join("app").join("data.json"), b"lost").expect_err("fails");
        assert!(error.to_string().starts_with("cannot create the folder: "), "{error}");
    }

    #[test]
    fn keeps_the_previous_file_when_the_temporary_file_cannot_be_written() {
        let folder = Folder::new();
        write_atomically(&folder.file(), b"before").expect("write the first");
        fs::create_dir(folder.temporary()).expect("put a folder in the temporary's place");
        let error = write_atomically(&folder.file(), b"after").expect_err("fails");
        assert!(error.to_string().starts_with("cannot write the temporary file: "), "{error}");
        assert_eq!(fs::read(folder.file()).expect("read the file"), b"before");
    }

    #[test]
    fn removes_the_temporary_file_when_it_cannot_replace_the_file() {
        let folder = Folder::new();
        fs::create_dir(folder.file()).expect("put a folder in the file's place");
        fs::write(folder.file().join("inside"), "kept").expect("fill the folder");
        let error = write_atomically(&folder.file(), b"lost").expect_err("fails");
        assert!(error.to_string().starts_with("cannot replace the file: "), "{error}");
        assert_eq!(fs::read_to_string(folder.file().join("inside")).expect("read the kept file"), "kept");
        assert!(!folder.temporary().exists());
    }

    #[rstest]
    #[case::temporary_file(".data.json.tmp")]
    #[case::file("data.json")]
    fn keeps_the_kind_of_the_error(#[case] blocked: &str) {
        let folder = Folder::new();
        fs::create_dir(folder.dir.path().join(blocked)).expect("put a folder in the way");
        fs::write(folder.dir.path().join(blocked).join("inside"), "kept").expect("fill the folder");
        let error = write_atomically(&folder.file(), b"lost").expect_err("fails");
        assert_ne!(error.kind(), ErrorKind::Other, "{error}");
    }
}
