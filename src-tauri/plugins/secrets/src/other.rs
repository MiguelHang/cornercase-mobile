use std::sync::Arc;

use cornercase_core::error::Error;
use cornercase_core::secrets::Secrets;
use tauri::plugin::Builder;
use tauri::{Manager, Runtime};

use crate::SecretStore;

const NO_STORE: &str = "no secret store on this platform yet";

struct Unavailable;

impl Secrets for Unavailable {
    fn get(&self, _: &str) -> Result<Option<Vec<u8>>, Error> {
        Err(unavailable())
    }

    fn set(&self, _: &str, _: &[u8]) -> Result<(), Error> {
        Err(unavailable())
    }

    fn delete(&self, _: &str) -> Result<(), Error> {
        Err(unavailable())
    }
}

fn unavailable() -> Error {
    Error::Secrets(NO_STORE.to_owned())
}

pub(crate) fn managing<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.setup(|app, _| {
        app.manage(SecretStore(Arc::new(Unavailable)));
        Ok(())
    })
}
