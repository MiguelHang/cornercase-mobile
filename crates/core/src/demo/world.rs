use std::collections::HashMap;
use std::time::SystemTime;

use tokio::sync::watch;
use tokio::time::Instant;

use super::data;
use crate::cli::contract::served;
use crate::cli::status::Snapshot;

const BOOT: &str = "demo";

pub(crate) struct World {
    pub(super) session: Snapshot,
    pub(super) changed: HashMap<u64, SystemTime>,
    pub(super) replies: Vec<String>,
    pub(super) turns: HashMap<u64, u64>,
    pub(super) due: HashMap<u64, Instant>,
    pub(super) told: watch::Sender<u64>,
    entered: SystemTime,
    started: Instant,
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
        Self {
            told: watch::Sender::new(session.revision),
            session,
            changed,
            replies: data::replies(),
            turns: HashMap::new(),
            due: HashMap::new(),
            entered,
            started: Instant::now(),
        }
    }
}

impl World {
    pub(super) fn contract(&self) -> String {
        served::descriptor(&self.session.boot).to_string()
    }

    pub(super) fn now(&self) -> SystemTime {
        self.entered + self.started.elapsed()
    }
}
