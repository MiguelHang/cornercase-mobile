use std::sync::Arc;

use cornercase_core::identity::Identity;
use keyring_core::mock;
use rstest::rstest;

use super::*;

const KEY: &str = "a/key";
const SECRET: &[u8] = &[0xff, 0xfe, 0x9d, 0x61, 0xb1];
const SECRET_IN_DEBUG: &str = "255, 254, 157, 97, 177";

fn mock_store() -> Arc<CredentialStore> {
    mock::Store::new().expect("a mock store")
}

fn on_mock(store: Arc<CredentialStore>) -> KeychainSecrets {
    KeychainSecrets::new(Ok(store), Modifiers::new())
}

fn secrets() -> (Arc<CredentialStore>, KeychainSecrets) {
    let store = mock_store();
    (Arc::clone(&store), on_mock(store))
}

fn fail_next_call(store: &Arc<CredentialStore>, error: keyring_core::Error) {
    let entry = store.build(SERVICE, KEY, None).expect("a mock entry");
    let credential: &mock::Cred = entry.as_any().downcast_ref().expect("a mock credential");
    credential.set_error(error);
}

mod get {
    use super::*;

    #[test]
    fn is_none_for_a_key_that_was_never_set() {
        let (_, secrets) = secrets();
        assert_eq!(secrets.get(KEY), Ok(None));
    }

    #[rstest]
    #[case::text(b"a token")]
    #[case::bytes_that_are_not_utf8(SECRET)]
    fn gives_back_the_bytes_that_were_set(#[case] value: &[u8]) {
        let (_, secrets) = secrets();
        secrets.set(KEY, value).expect("the mock takes the secret");
        assert_eq!(secrets.get(KEY), Ok(Some(value.to_vec())));
    }
}

mod delete {
    use super::*;

    #[test]
    fn is_ok_for_a_key_that_was_never_set() {
        let (_, secrets) = secrets();
        assert_eq!(secrets.delete(KEY), Ok(()));
    }

    #[test]
    fn removes_the_secret() {
        let (_, secrets) = secrets();
        secrets.set(KEY, SECRET).expect("the mock takes the secret");
        secrets.delete(KEY).expect("the mock deletes the secret");
        assert_eq!(secrets.get(KEY), Ok(None));
    }
}

mod failure {
    use super::*;

    #[rstest]
    #[case::get(|secrets: &KeychainSecrets| secrets.get(KEY).map(drop))]
    #[case::set(|secrets: &KeychainSecrets| secrets.set(KEY, SECRET))]
    #[case::delete(|secrets: &KeychainSecrets| secrets.delete(KEY))]
    fn becomes_a_secrets_error(#[case] operation: fn(&KeychainSecrets) -> Result<(), Error>) {
        let (store, secrets) = secrets();
        fail_next_call(&store, keyring_core::Error::NoStorageAccess("locked".into()));
        assert_eq!(operation(&secrets), Err(Error::Secrets("Couldn't access platform storage: locked".to_owned())));
    }

    #[rstest]
    #[case::bad_encoding(|_: &CredentialStore| keyring_core::Error::BadEncoding(SECRET.to_vec()))]
    #[case::bad_data_format(|_: &CredentialStore| keyring_core::Error::BadDataFormat(SECRET.to_vec(), "garbled".into()))]
    #[case::ambiguous(ambiguous)]
    fn keeps_the_secret_out_of_the_message(#[case] error: fn(&CredentialStore) -> keyring_core::Error) {
        let (store, secrets) = secrets();
        fail_next_call(&store, error(&*store));
        let Err(Error::Secrets(message)) = secrets.get(KEY) else {
            panic!("the mock's error should become a secrets error");
        };
        assert!(!message.contains(SECRET_IN_DEBUG), "{message}");
    }

    fn ambiguous(store: &CredentialStore) -> keyring_core::Error {
        let other = store.build(SERVICE, "another/key", None).expect("a mock entry");
        other.set_secret(SECRET).expect("the mock takes the secret");
        keyring_core::Error::Ambiguous(vec![other])
    }
}

mod modifiers {
    use super::*;

    #[test]
    fn reach_the_store_with_each_entry() {
        let secrets = KeychainSecrets::new(Ok(mock_store()), Modifiers::from([("access-policy", "a policy")]));
        assert_eq!(
            secrets.get(KEY),
            Err(Error::Secrets("Unsupported: The mock store doesn't allow entry modifiers".to_owned()))
        );
    }
}

mod store {
    use super::*;

    #[cfg(not(target_os = "ios"))]
    #[test]
    fn fails_with_a_clear_message_on_a_platform_without_one() {
        assert_eq!(store().get(KEY), Err(Error::Secrets(NO_STORE.to_owned())));
    }

    #[cfg(target_os = "ios")]
    #[test]
    fn keeps_items_on_this_device_once_it_was_unlocked() {
        let store = store();
        assert!(store.store.is_ok());
        assert_eq!(store.modifiers, Modifiers::from([("access-policy", "AfterFirstUnlockThisDeviceOnly")]));
    }
}

mod identity {
    use super::*;

    #[test]
    fn keeps_its_fingerprint_from_one_start_to_the_next() {
        let store = mock_store();
        let first = Identity::load_or_create(&on_mock(Arc::clone(&store))).expect("a new identity");
        let second = Identity::load_or_create(&on_mock(store)).expect("the saved identity");
        assert_eq!(first.fingerprint(), second.fingerprint());
    }
}
