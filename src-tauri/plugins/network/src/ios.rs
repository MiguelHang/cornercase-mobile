use std::error::Error;

use cornercase_core::network::Network;
use serde::Serialize;
use serde::de::IgnoredAny;
use tauri::Runtime;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::plugin::{Builder, PluginApi};

tauri::ios_plugin_binding!(init_plugin_network);

#[derive(Serialize)]
struct Watch {
    channel: Channel,
}

pub(crate) fn watching<R: Runtime>(
    builder: Builder<R>,
    on_path: impl Fn(Network) + Send + Sync + 'static,
) -> Builder<R> {
    builder.setup(move |_, api| watch(&api, on_path))
}

fn watch<R: Runtime>(
    api: &PluginApi<R, ()>,
    on_path: impl Fn(Network) + Send + Sync + 'static,
) -> Result<(), Box<dyn Error>> {
    let handle = api.register_ios_plugin(init_plugin_network)?;
    let channel = Channel::new(move |body: InvokeResponseBody| {
        on_path(body.deserialize()?);
        Ok(())
    });
    handle.run_mobile_plugin::<IgnoredAny>("watch", Watch { channel })?;
    Ok(())
}
