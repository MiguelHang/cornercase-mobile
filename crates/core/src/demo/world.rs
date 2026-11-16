use std::collections::HashMap;
use std::time::SystemTime;

use super::data;
use crate::cli::contract::served;
use crate::cli::status::Snapshot;

const BOOT: &str = "demo";

pub(crate) struct World {
    pub(super) session: Snapshot,
    pub(super) changed: HashMap<u64, SystemTime>,
}

impl Default for World {
    fn default() -> Self {
        let entered = SystemTime::now();
        let session = Snapshot {
            boot: BOOT.to_owned(),
            revision: 1,
            version: served::version().to_owned(),
            groups: Vec::new(),
            projects: data::projects(),
        };
        let changed = data::ages().map(|(pane, ago)| (pane, entered.checked_sub(ago).unwrap_or(entered))).collect();
        Self { session, changed }
    }
}

impl World {
    pub(super) fn contract(&self) -> String {
        served::descriptor(&self.session.boot).to_string()
    }
}
