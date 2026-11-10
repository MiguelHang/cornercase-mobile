use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::plugin::mobile::PluginInvokeError;
use tauri::plugin::{Builder, PluginHandle};
use tauri::{Manager, Runtime};

use crate::SecretStore;
use crate::bridge::{Bridge, Commands};

const IDENTIFIER: &str = "dev.usecornercase.secrets";
const CLASS: &str = "SecretsPlugin";
const UNEXPECTED_SHAPE: &str = "the keystore plugin exchanged a message of an unexpected shape";

#[derive(Serialize)]
struct Name<'a> {
    name: &'a str,
}

#[derive(Serialize)]
struct Entry<'a> {
    name: &'a str,
    value: &'a str,
}

#[derive(Deserialize)]
struct Found {
    value: Option<String>,
}

struct Plugin<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Commands for Plugin<R> {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        self.0.run_mobile_plugin::<Found>("get", Name { name }).map(|found| found.value).map_err(reason)
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        self.0.run_mobile_plugin("set", Entry { name, value }).map_err(reason)
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.0.run_mobile_plugin("delete", Name { name }).map_err(reason)
    }
}

fn reason(error: PluginInvokeError) -> String {
    match error {
        PluginInvokeError::CannotDeserializeResponse(_) | PluginInvokeError::CannotSerializePayload(_) => {
            UNEXPECTED_SHAPE.to_owned()
        }
        other => other.to_string(),
    }
}

pub(crate) fn managing<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.setup(|app, api| {
        let plugin = Plugin(api.register_android_plugin(IDENTIFIER, CLASS)?);
        app.manage(SecretStore(Arc::new(Bridge::new(plugin))));
        Ok(())
    })
}
