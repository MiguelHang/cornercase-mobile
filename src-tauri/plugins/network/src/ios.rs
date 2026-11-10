use tauri::Runtime;
use tauri::plugin::mobile::PluginInvokeError;
use tauri::plugin::{PluginApi, PluginHandle};

tauri::ios_plugin_binding!(init_plugin_network);

pub(crate) fn registered<R: Runtime>(api: &PluginApi<R, ()>) -> Result<PluginHandle<R>, PluginInvokeError> {
    api.register_ios_plugin(init_plugin_network)
}
