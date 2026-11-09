use std::fmt::Write as _;

use cornercase_core::secrets::Secrets;
use rstest::rstest;

use super::*;
use crate::secrets::{mock_store, on_mock};

mod line {
    use super::*;

    #[test]
    fn names_the_fingerprint_of_a_new_identity() {
        let secrets = on_mock(mock_store());
        let identity = Identity::load_or_create(&secrets).expect("a new identity");
        let fingerprint = identity.fingerprint();
        assert_eq!(line(Ok(identity)), format!("cornercase-mobile: identity {fingerprint}"));
    }

    #[test]
    fn holds_neither_the_seed_nor_the_whole_key() {
        let secrets = on_mock(mock_store());
        let identity = Identity::load_or_create(&secrets).expect("a new identity");
        let public_key = identity.public_key();
        let seed = secrets.get("identity/ed25519").expect("the mock reads").expect("the saved seed");
        let line = line(Ok(identity));
        let seed_in_hex = seed.iter().fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").expect("a string takes the byte");
            text
        });
        for secret in [seed_in_hex, public_key] {
            assert!(!line.contains(&secret), "{line}");
        }
    }

    #[rstest]
    #[case::store(Error::Secrets("locked".to_owned()), "the secret store failed: locked")]
    #[case::seed(Error::Identity, "the saved identity is not a 32-byte Ed25519 seed")]
    fn says_why_there_is_no_identity(#[case] error: Error, #[case] reason: &str) {
        assert_eq!(line(Err(error)), format!("cornercase-mobile: no identity: {reason}"));
    }

    #[cfg(not(target_os = "ios"))]
    #[test]
    fn says_there_is_no_store_on_this_platform() {
        assert_eq!(
            line(Identity::load_or_create(&secrets::store())),
            "cornercase-mobile: no identity: the secret store failed: no secret store on this platform yet"
        );
    }
}
