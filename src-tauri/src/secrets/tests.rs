use std::path::PathBuf;
use std::sync::Arc;

use cornercase_core::secrets::hex::encoded;
use cornercase_core::test_util::TempDir;
use rstest::rstest;

use super::keychain::Modifiers;
use super::*;

const KEY: &str = "a/key";
const SECRET: &[u8] = &[0xff, 0xfe, 0x9d, 0x61, 0xb1];
const BROKEN: &str = "the store is broken";

fn given_store() -> Arc<dyn Secrets> {
    Arc::new(on_mock(mock_store()))
}

fn given_broken_store() -> Arc<dyn Secrets> {
    Arc::new(KeychainSecrets::new(Err(Error::Secrets(BROKEN.to_owned())), Modifiers::new()))
}

struct Folder {
    dir: TempDir,
}

impl Folder {
    fn new() -> Self {
        Self { dir: TempDir::new() }
    }

    fn holding_a_secret() -> Self {
        let folder = Self::new();
        folder.file().set(KEY, SECRET).expect("the file takes the secret");
        folder
    }

    fn path(&self) -> PathBuf {
        self.dir.path().join(ROUTES_FILE)
    }

    fn file(&self) -> FileSecrets {
        FileSecrets::new(self.path())
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod store {
    use super::*;

    #[test]
    fn fails_with_a_clear_message_on_a_platform_without_one() {
        let app = tauri::test::mock_app();
        assert_eq!(store(app.handle()).get(KEY), Err(Error::Secrets(NO_STORE.to_owned())));
    }
}

mod routes {
    use super::*;

    #[test]
    fn share_the_store_of_the_identity() {
        let dir = TempDir::new();
        let store = given_store();
        assert!(Arc::ptr_eq(&routes(Some(dir.path()), Arc::clone(&store)), &store));
    }

    #[test]
    fn share_the_store_of_the_identity_without_a_data_folder() {
        let store = given_store();
        assert!(Arc::ptr_eq(&routes(None, Arc::clone(&store)), &store));
    }

    #[cfg(target_os = "android")]
    #[test]
    fn take_the_secrets_of_the_old_file_into_the_store() {
        let folder = Folder::holding_a_secret();
        let store = given_store();
        assert!(Arc::ptr_eq(&routes(Some(folder.dir.path()), Arc::clone(&store)), &store));
        assert_eq!(store.get(KEY), Ok(Some(SECRET.to_vec())));
        assert!(!folder.path().exists());
    }
}

mod moved {
    use super::*;

    #[test]
    fn hands_over_the_store_when_there_is_no_file() {
        let folder = Folder::new();
        let store = given_store();
        assert!(Arc::ptr_eq(&moved(folder.file(), Arc::clone(&store)), &store));
    }

    #[test]
    fn copies_the_file_into_the_store_and_removes_it() {
        let folder = Folder::holding_a_secret();
        let store = given_store();
        assert!(Arc::ptr_eq(&moved(folder.file(), Arc::clone(&store)), &store));
        assert_eq!(store.get(KEY), Ok(Some(SECRET.to_vec())));
        assert!(!folder.path().exists());
    }

    #[test]
    fn keeps_using_the_file_when_the_store_fails() {
        let folder = Folder::holding_a_secret();
        let routes = moved(folder.file(), given_broken_store());
        assert_eq!(routes.get(KEY), Ok(Some(SECRET.to_vec())));
        assert!(folder.path().is_file());
    }

    #[test]
    fn tries_again_on_the_next_start() {
        let folder = Folder::holding_a_secret();
        moved(folder.file(), given_broken_store());
        let store = given_store();
        assert!(Arc::ptr_eq(&moved(folder.file(), Arc::clone(&store)), &store));
        assert_eq!(store.get(KEY), Ok(Some(SECRET.to_vec())));
    }
}

mod line {
    use super::*;

    #[rstest]
    #[case::nothing_without_a_file(Ok(Migration::NoFile), None)]
    #[case::how_many_moved(Ok(Migration::Moved(2)), Some("cornercase-mobile: moved 2 route secrets into the store"))]
    #[case::why_the_file_stays(
        Err(Error::Secrets(BROKEN.to_owned())),
        Some("cornercase-mobile: the route secrets stay in their file: the secret store failed: the store is broken")
    )]
    fn says(#[case] migration: Result<Migration, Error>, #[case] said: Option<&str>) {
        assert_eq!(line(&migration).as_deref(), said);
    }

    #[test]
    fn keeps_the_secret_out_of_a_failure() {
        let folder = Folder::holding_a_secret();
        let said = line(&migrate(&folder.file(), &*given_broken_store())).expect("a line about the failure");
        assert!(said.starts_with("cornercase-mobile: the route secrets stay in their file: "));
        assert!(!said.contains(&encoded(SECRET)));
    }
}
