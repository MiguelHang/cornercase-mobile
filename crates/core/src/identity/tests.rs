use ed25519_dalek::VerifyingKey;
use rstest::rstest;

use super::*;
use crate::secrets::memory::MemorySecrets;

const SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const PUBLIC_KEY: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len()).step_by(2).map(|at| u8::from_str_radix(&hex[at..at + 2], 16).expect("hex digits")).collect()
}

fn rfc_identity() -> Identity {
    Identity::from_seed(&bytes(SEED)).expect("a 32-byte seed")
}

mod from_seed {
    use super::*;

    #[rstest]
    #[case::empty(0)]
    #[case::one_byte_short(31)]
    #[case::one_byte_long(33)]
    fn refuses_a_seed_that_is_not_32_bytes(#[case] length: usize) {
        let error = Identity::from_seed(&vec![7; length]).expect_err("a seed of the wrong length");
        assert_eq!(error, Error::Identity);
    }
}

mod load_or_create {
    use super::*;

    fn saved(secrets: &MemorySecrets) -> Option<Vec<u8>> {
        secrets.get(KEY).expect("the memory store reads")
    }

    #[test]
    fn saves_a_new_32_byte_seed_under_its_key() {
        let secrets = MemorySecrets::default();
        let identity = Identity::load_or_create(&secrets).expect("a new identity");
        let seed = saved(&secrets).expect("a saved seed");
        assert_eq!(seed.len(), 32);
        assert_eq!(Identity::from_seed(&seed).expect("the saved seed").public_key(), identity.public_key());
    }

    #[test]
    fn gives_the_same_key_the_second_time_without_writing() {
        let secrets = MemorySecrets::default();
        let first = Identity::load_or_create(&secrets).expect("a new identity");
        secrets.fail_writes(true);
        let second = Identity::load_or_create(&secrets).expect("the saved identity");
        assert_eq!(second.public_key(), first.public_key());
    }

    #[test]
    fn uses_the_saved_seed() {
        let secrets = MemorySecrets::default();
        secrets.set(KEY, &bytes(SEED)).expect("the memory store writes");
        let identity = Identity::load_or_create(&secrets).expect("the saved identity");
        assert_eq!(identity.public_key(), "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo");
    }

    #[test]
    fn keeps_a_saved_seed_that_is_not_32_bytes() {
        let secrets = MemorySecrets::default();
        secrets.set(KEY, &[7; 31]).expect("the memory store writes");
        let error = Identity::load_or_create(&secrets).expect_err("a short seed");
        assert_eq!(error, Error::Identity);
        assert_eq!(saved(&secrets), Some(vec![7; 31]));
    }

    #[test]
    fn saves_nothing_when_the_store_refuses_writes() {
        let secrets = MemorySecrets::default();
        secrets.fail_writes(true);
        let error = Identity::load_or_create(&secrets).expect_err("a refused write");
        assert!(matches!(error, Error::Secrets(_)), "{error}");
        assert_eq!(saved(&secrets), None);
    }
}

mod public_key {
    use super::*;

    #[test]
    fn is_the_rfc_8032_key_in_base64url() {
        assert_eq!(rfc_identity().public_key(), "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo");
    }

    #[test]
    fn decodes_to_the_verifying_key() {
        let text = rfc_identity().public_key();
        assert_eq!(text.len(), 43);
        let decoded = URL_SAFE_NO_PAD.decode(&text).expect("base64url");
        let key = VerifyingKey::try_from(decoded.as_slice()).expect("a verifying key");
        assert_eq!(key.as_bytes().as_slice(), bytes(PUBLIC_KEY));
    }
}

mod fingerprint {
    use super::*;

    #[test]
    fn is_the_first_five_bytes_of_the_public_key_in_hex() {
        assert_eq!(rfc_identity().fingerprint(), "d75a980182");
    }
}

mod debug {
    use super::*;

    #[test]
    fn leaves_the_seed_and_the_public_key_out() {
        let identity = rfc_identity();
        let text = format!("{identity:?}");
        assert!(text.starts_with("Identity"), "{text}");
        assert!(!text.contains(SEED), "{text}");
        assert!(!text.contains(PUBLIC_KEY), "{text}");
        assert!(!text.contains(&identity.public_key()), "{text}");
    }
}
