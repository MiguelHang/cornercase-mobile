use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use tokio::sync::watch;
use tokio::time::Instant;

use super::changes::{self, Diffs};
use super::data;
use super::files::Tree;
use super::question::Question;
use super::surface::Row;
use crate::board::places;
use crate::cli::contract::served;
use crate::cli::status::Snapshot;
use crate::cli::surface::Size;

const BOOT: &str = "demo";

pub(crate) struct World {
    pub(super) session: Snapshot,
    pub(super) changed: HashMap<u64, SystemTime>,
    pub(super) replies: Vec<String>,
    pub(super) turns: HashMap<u64, u64>,
    pub(super) due: HashMap<u64, (Instant, &'static str)>,
    pub(super) questions: Vec<Question>,
    pub(super) printed: HashMap<u64, Vec<Row>>,
    pub(super) views: HashMap<u64, Size>,
    pub(super) trees: Vec<Tree>,
    pub(super) diffs: Vec<Diffs>,
    pub(super) seen: HashSet<(u64, String)>,
    pub(super) told: watch::Sender<u64>,
    issued: u64,
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
        let trees = data::trees();
        let diffs = changes::diffs(data::changes(), &trees);
        let issued =
            places(&session).flat_map(|place| [place.project.id, place.workspace.id, place.tab.id, place.pane.id]);
        let issued = issued.max().unwrap_or_default();
        let changed = data::ages().map(|(pane, ago)| (pane, entered.checked_sub(ago).unwrap_or(entered))).collect();
        Self {
            told: watch::Sender::new(session.revision),
            session,
            changed,
            replies: data::replies(),
            turns: HashMap::new(),
            due: HashMap::new(),
            questions: data::questions(),
            printed: data::terminals(),
            views: HashMap::new(),
            trees,
            diffs,
            seen: HashSet::new(),
            issued,
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

    pub(super) fn revised(&mut self) {
        self.session.revision += 1;
        self.told.send_replace(self.session.revision);
    }

    pub(super) fn issue(&mut self) -> u64 {
        self.issued += 1;
        self.issued
    }
}
