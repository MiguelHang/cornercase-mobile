use tauri::Runtime;
use tauri::plugin::mobile::PluginInvokeError;
use tauri::plugin::{PluginApi, PluginHandle};

const IDENTIFIER: &str = "dev.usecornercase.network";
const CLASS: &str = "NetworkPlugin";

pub(crate) fn registered<R: Runtime>(api: &PluginApi<R, ()>) -> Result<PluginHandle<R>, PluginInvokeError> {
    api.register_android_plugin(IDENTIFIER, CLASS)
}
