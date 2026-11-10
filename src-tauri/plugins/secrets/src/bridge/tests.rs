use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use rstest::rstest;

use super::*;

const NAME: &str = "identity/ed25519";
const SECRET_WORD: &str = "do-not-repeat-me";
const REASON: &str = "the keystore key is gone";

#[derive(Default)]
struct FakePlugin {
    values: Mutex<HashMap<String, String>>,
    refusing: bool,
}

impl FakePlugin {
    fn values(&self) -> MutexGuard<'_, HashMap<String, String>> {
        self.values.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn answer(&self) -> Result<MutexGuard<'_, HashMap<String, String>>, String> {
        if self.refusing {
            return Err(REASON.to_owned());
        }
        Ok(self.values())
    }
}

impl Commands for FakePlugin {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        Ok(self.answer()?.get(name).cloned())
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        self.answer()?.insert(name.to_owned(), value.to_owned());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.answer()?.remove(name);
        Ok(())
    }
}

fn bridge() -> Bridge<FakePlugin> {
    Bridge::new(FakePlugin::default())
}

fn refusing() -> Bridge<FakePlugin> {
    Bridge::new(FakePlugin { refusing: true, ..FakePlugin::default() })
}

fn holding(reply: &str) -> Bridge<FakePlugin> {
    let bridge = bridge();
    bridge.0.values().insert(NAME.to_owned(), reply.to_owned());
    bridge
}

fn message(error: Error) -> String {
    let Error::Secrets(message) = error else { panic!("expected a secrets error, got {error:?}") };
    message
}

mod get {
    use super::*;

    #[test]
    fn a_name_that_was_never_set_is_none() {
        assert_eq!(bridge().get(NAME), Ok(None));
    }

    #[rstest]
    #[case::text(SECRET_WORD.as_bytes().to_vec())]
    #[case::bytes_that_are_not_utf8(vec![0x00, 0xff, 0xfe, 0x80, 0x0a])]
    #[case::nothing(Vec::new())]
    fn reads_what_was_set(#[case] value: Vec<u8>) {
        let bridge = bridge();
        bridge.set(NAME, &value).expect("set the secret");
        assert_eq!(bridge.get(NAME), Ok(Some(value)));
    }
}

mod set {
    use super::*;

    #[test]
    fn hands_the_plugin_the_value_in_hex() {
        let bridge = bridge();
        bridge.set(NAME, &[0x00, 0xff, 0x80, 0x0a]).expect("set the secret");
        assert_eq!(bridge.0.values().get(NAME).map(String::as_str), Some("00ff800a"));
    }

    #[test]
    fn replaces_the_value_of_the_name() {
        let bridge = bridge();
        bridge.set(NAME, b"first").expect("set the first value");
        bridge.set(NAME, b"second").expect("set the second value");
        assert_eq!(bridge.get(NAME), Ok(Some(b"second".to_vec())));
    }
}

mod delete {
    use super::*;

    #[test]
    fn a_name_that_was_never_set_is_fine() {
        assert_eq!(bridge().delete(NAME), Ok(()));
    }

    #[test]
    fn forgets_the_value() {
        let bridge = bridge();
        bridge.set(NAME, b"seed").expect("set the secret");
        bridge.delete(NAME).expect("delete the secret");
        assert_eq!(bridge.get(NAME), Ok(None));
    }
}

mod failing {
    use super::*;

    fn fails(operation: fn(&dyn Secrets) -> Result<(), Error>, bridge: &Bridge<FakePlugin>) -> String {
        message(operation(bridge).expect_err("the operation fails"))
    }

    #[rstest]
    #[case::reading(|store: &dyn Secrets| store.get(NAME).map(drop), "cannot read a secret from the keystore")]
    #[case::writing(|store: &dyn Secrets| store.set(NAME, SECRET_WORD.as_bytes()), "cannot write a secret to the keystore")]
    #[case::deleting(|store: &dyn Secrets| store.delete(NAME), "cannot delete a secret from the keystore")]
    fn a_refusal_names_the_operation_and_the_reason(
        #[case] operation: fn(&dyn Secrets) -> Result<(), Error>,
        #[case] what: &str,
    ) {
        assert_eq!(fails(operation, &refusing()), format!("{what}: {REASON}"));
    }

    #[test]
    fn a_refused_write_leaves_out_the_value() {
        let message = fails(|store| store.set(NAME, SECRET_WORD.as_bytes()), &refusing());
        assert!(!message.contains(SECRET_WORD), "{message}");
        assert!(!message.contains(&hex::encoded(SECRET_WORD.as_bytes())), "{message}");
    }

    #[rstest]
    #[case::not_hex(SECRET_WORD)]
    #[case::an_odd_length("abc")]
    fn a_reply_that_is_not_hex_is_an_error_without_it(#[case] reply: &str) {
        let message = fails(|store| store.get(NAME).map(drop), &holding(reply));
        assert_eq!(message, "the keystore returned a secret that is not hex");
    }
}
