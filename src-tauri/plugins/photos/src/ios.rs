use cornercase_core::cli::paste_file::{Picked, Picking};
use cornercase_core::error::Error;
use tauri::plugin::{Builder, PluginHandle};
use tauri::{Manager, Runtime};

tauri::ios_plugin_binding!(init_plugin_photos);

pub struct Photos<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Photos<R> {
    pub fn pick(&self) -> Result<Option<Picked>, Error> {
        self.0
            .run_mobile_plugin::<Picking>("pick", ())
            .map(Picking::picked)
            .map_err(|error| Error::Picker(error.to_string()))
    }
}

pub(crate) fn managing<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.setup(|app, api| {
        app.manage(Photos(api.register_ios_plugin(init_plugin_photos)?));
        Ok(())
    })
}
