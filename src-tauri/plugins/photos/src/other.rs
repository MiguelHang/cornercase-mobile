use std::marker::PhantomData;

use cornercase_core::cli::paste_file::Picked;
use cornercase_core::error::Error;
use tauri::plugin::Builder;
use tauri::{Manager, Runtime};

const UNAVAILABLE: &str = "only the iOS app has a photo picker";

pub struct Photos<R: Runtime>(PhantomData<fn() -> R>);

impl<R: Runtime> Photos<R> {
    pub fn pick(&self) -> Result<Option<Picked>, Error> {
        Err(Error::Picker(UNAVAILABLE.to_owned()))
    }
}

pub(crate) fn managing<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.setup(|app, _| {
        app.manage(Photos::<R>(PhantomData));
        Ok(())
    })
}
