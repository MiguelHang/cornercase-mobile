use std::collections::HashMap;
use std::sync::Arc;

use cornercase_core::error::Error;

use super::*;
use crate::commands::error::ErrorCode;
use crate::secrets::KeychainSecrets;
use crate::test_util::Studio;

const NO_FOLDER: Result<std::path::PathBuf, &str> = Err("no home folder");

mod the_fingerprint {
    use super::*;

    #[test]
    fn is_the_loaded_identity_s() {
        let studio = Studio::new();
        let loaded = studio.state.identity().map(Identity::fingerprint);
        assert!(loaded.is_ok(), "{loaded:?}");
        assert_eq!(fingerprint(&studio.state), loaded);
    }

    #[test]
    fn says_why_there_is_none() {
        let locked = KeychainSecrets::new(Err(Error::Secrets("locked".to_owned())), HashMap::new());
        let state = AppState::load(NO_FOLDER, Arc::new(locked));
        assert_eq!(fingerprint(&state).map_err(|error| error.code), Err(ErrorCode::NoIdentity));
    }
}
