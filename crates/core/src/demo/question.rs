use serde::Deserialize;

use super::surface::{Row, Tone, row};
use super::world::World;

const RULE: char = '─';
const OPEN: char = '☐';
const TICKED: char = '✔';
const POINTER: char = '❯';
const DETAIL: &str = "     ";
const DONE: &str = "done";

#[derive(Deserialize)]
pub(super) struct Question {
    pane: u64,
    said: Vec<String>,
    topic: String,
    asked: String,
    options: Vec<Choice>,
    help: String,
    #[serde(skip)]
    chosen: Option<usize>,
}

#[derive(Deserialize)]
struct Choice {
    label: String,
    detail: String,
}

impl Question {
    fn rows(&self, cols: u16) -> Vec<Row> {
        let mut rows: Vec<Row> = self.said.iter().map(|line| row(Tone::Plain, line.as_str())).collect();
        rows.push(blank());
        match self.chosen.and_then(|at| self.options.get(at)) {
            Some(choice) => rows.push(row(Tone::Dim, format!("{TICKED} {}: {}", self.topic, choice.label))),
            None => rows.extend(self.open(cols)),
        }
        rows
    }

    fn open(&self, cols: u16) -> Vec<Row> {
        let rule = row(Tone::Dim, RULE.to_string().repeat(usize::from(cols)));
        let chip = row(Tone::Chip, format!(" {OPEN} {} ", self.topic));
        let mut rows = vec![rule, chip, blank(), row(Tone::Asked, self.asked.as_str())];
        rows.push(blank());
        for (at, choice) in self.options.iter().enumerate() {
            let (tone, mark) = if at == 0 { (Tone::Picked, POINTER) } else { (Tone::Plain, ' ') };
            rows.push(row(tone, format!("{mark} {}. {}", at + 1, choice.label)));
            rows.push(row(Tone::Dim, format!("{DETAIL}{}", choice.detail)));
        }
        rows.extend([blank(), row(Tone::Dim, self.help.as_str())]);
        rows
    }

    fn option(&self, key: &str) -> Option<usize> {
        let at = key.parse::<usize>().ok()?.checked_sub(1)?;
        (at < self.options.len()).then_some(at)
    }
}

impl World {
    pub(super) fn pressed(&mut self, pane: u64, key: Option<&str>) {
        let open = self.questions.iter_mut().find(|question| question.pane == pane && question.chosen.is_none());
        let Some(question) = open else {
            return;
        };
        if let Some(at) = key.and_then(|key| question.option(key)) {
            question.chosen = Some(at);
            self.worked(pane, DONE);
        }
    }

    pub(super) fn asked(&self, pane: u64, cols: u16) -> Option<Vec<Row>> {
        self.questions.iter().find(|question| question.pane == pane).map(|question| question.rows(cols))
    }
}

fn blank() -> Row {
    Row::new()
}

#[cfg(test)]
mod tests;
