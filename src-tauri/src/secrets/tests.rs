use std::sync::Arc;

use cornercase_bridge::test_util::TempDir;

use super::*;

#[cfg(not(target_os = "ios"))]
const KEY: &str = "a/key";
#[cfg(target_os = "android")]
const SECRET: &[u8] = &[0xff, 0xfe, 0x9d, 0x61, 0xb1];

fn given_store() -> Arc<dyn Secrets> {
    Arc::new(on_mock(mock_store()))
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

    #[cfg(not(target_os = "android"))]
    #[test]
    fn share_the_store_of_the_identity() {
        let dir = TempDir::new();
        let store = given_store();
        assert!(Arc::ptr_eq(&routes(Some(dir.path()), Arc::clone(&store)), &store));
    }

    #[cfg(target_os = "android")]
    #[test]
    fn are_kept_in_a_file_in_the_data_folder() {
        let dir = TempDir::new();
        routes(Some(dir.path()), given_store()).set(KEY, SECRET).expect("the file takes the secret");
        assert!(dir.path().join(ROUTES_FILE).is_file());
        assert_eq!(routes(Some(dir.path()), given_store()).get(KEY), Ok(Some(SECRET.to_vec())));
    }

    #[cfg(target_os = "android")]
    #[test]
    fn share_the_store_of_the_identity_without_a_data_folder() {
        let store = given_store();
        assert!(Arc::ptr_eq(&routes(None, Arc::clone(&store)), &store));
    }
}
