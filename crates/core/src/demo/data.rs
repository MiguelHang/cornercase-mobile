use std::time::Duration;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::cli::status::Project;

const PROJECTS: &str = include_str!("data/projects.json");
const AGES: &str = include_str!("data/ages.json");

#[derive(Deserialize)]
struct Age {
    pane: u64,
    minutes: u64,
}

pub(super) fn projects() -> Vec<Project> {
    read(PROJECTS)
}

pub(super) fn ages() -> impl Iterator<Item = (u64, Duration)> {
    read::<Vec<Age>>(AGES).into_iter().map(|age| (age.pane, Duration::from_secs(age.minutes * 60)))
}

fn read<T: DeserializeOwned>(json: &str) -> T {
    serde_json::from_str(json).expect("the demo's data is valid")
}
