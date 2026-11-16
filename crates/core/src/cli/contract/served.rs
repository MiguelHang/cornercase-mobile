use serde_json::{Value, json};

use super::{CONTRACT, REQUIRED, Required, SINCE};

pub(crate) fn descriptor(boot: &str) -> Value {
    REQUIRED.described(newest(SINCE.iter().map(|(_, _, version)| *version)), boot)
}

impl Required {
    fn described(&self, version: &str, boot: &str) -> Value {
        let operations: Vec<Value> =
            self.operations.iter().map(|name| json!({ "name": name, "available": true })).collect();
        let streams: Vec<Value> =
            self.streams.iter().map(|(name, version)| json!({ "name": name, "version": version })).collect();
        json!({
            "version": version,
            "contract": CONTRACT,
            "boot": boot,
            "operations": operations,
            "features": self.features,
            "streams": streams,
        })
    }
}

fn newest<'a>(versions: impl Iterator<Item = &'a str>) -> &'a str {
    versions.max_by_key(|version| numbers(version)).unwrap_or_default()
}

fn numbers(version: &str) -> Vec<u32> {
    version.split('.').map(|part| part.parse().unwrap_or_default()).collect()
}

#[cfg(test)]
mod tests;
