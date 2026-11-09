use std::collections::HashMap;
use std::sync::Arc;

use cornercase_core::error::Error;

use super::*;
use crate::commands::error::ErrorCode;
use crate::secrets::KeychainSecrets;
use crate::test_util::{Ipc, Studio};

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

mod over_ipc {
    use serde_json::json;

    use super::*;

    #[test]
    fn answers_the_fingerprint() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        let answered = ipc.invoke("identity_fingerprint", json!({})).expect("a fingerprint");
        assert_eq!(answered.as_str().map(str::to_owned), fingerprint(&ipc.state()).ok());
    }

    #[test]
    fn says_why_there_is_no_fingerprint() {
        let locked = KeychainSecrets::new(Err(Error::Secrets("locked".to_owned())), HashMap::new());
        let ipc = Ipc::new(AppState::load(NO_FOLDER, Arc::new(locked)));
        let refused = ipc.invoke("identity_fingerprint", json!({})).expect_err("no identity");
        assert_eq!(refused["code"], "no_identity");
    }
}
