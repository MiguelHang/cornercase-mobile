use cornercase_core::network::Network;
use tauri::Runtime;
use tauri::plugin::Builder;

pub(crate) fn watching<R: Runtime>(builder: Builder<R>, _: impl Fn(Network) + Send + Sync + 'static) -> Builder<R> {
    builder
}
