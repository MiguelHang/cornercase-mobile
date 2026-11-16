use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::question::Question;
use super::surface::Row;
use crate::cli::status::Project;

const PROJECTS: &str = include_str!("data/projects.json");
const AGES: &str = include_str!("data/ages.json");
const REPLIES: &str = include_str!("data/replies.json");
const QUESTIONS: &str = include_str!("data/questions.json");
const TERMINALS: &str = include_str!("data/terminals.json");

#[derive(Deserialize)]
struct Age {
    pane: u64,
    minutes: u64,
}

#[derive(Deserialize)]
struct Printed {
    pane: u64,
    lines: Vec<Row>,
}

pub(super) fn projects() -> Vec<Project> {
    read(PROJECTS)
}

pub(super) fn ages() -> impl Iterator<Item = (u64, Duration)> {
    read::<Vec<Age>>(AGES).into_iter().map(|age| (age.pane, Duration::from_secs(age.minutes * 60)))
}

pub(super) fn replies() -> Vec<String> {
    read(REPLIES)
}

pub(super) fn questions() -> Vec<Question> {
    read(QUESTIONS)
}

pub(super) fn terminals() -> HashMap<u64, Vec<Row>> {
    read::<Vec<Printed>>(TERMINALS).into_iter().map(|printed| (printed.pane, printed.lines)).collect()
}

fn read<T: DeserializeOwned>(json: &str) -> T {
    serde_json::from_str(json).expect("the demo's data is valid")
}
