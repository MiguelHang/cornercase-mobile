use serde::Deserialize;

use super::surface::{ASKED, CHIP, DIM, PICKED, PLAIN, Row};
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
        let mut rows: Vec<Row> = self.said.iter().map(|line| (PLAIN, line.clone())).collect();
        rows.push(blank());
        match self.chosen.and_then(|at| self.options.get(at)) {
            Some(choice) => rows.push((DIM, format!("{TICKED} {}: {}", self.topic, choice.label))),
            None => rows.extend(self.open(cols)),
        }
        rows
    }

    fn open(&self, cols: u16) -> Vec<Row> {
        let rule = (DIM, RULE.to_string().repeat(usize::from(cols)));
        let mut rows = vec![rule, (CHIP, format!(" {OPEN} {} ", self.topic)), blank(), (ASKED, self.asked.clone())];
        rows.push(blank());
        for (at, choice) in self.options.iter().enumerate() {
            let (style, mark) = if at == 0 { (PICKED, POINTER) } else { (PLAIN, ' ') };
            rows.push((style, format!("{mark} {}. {}", at + 1, choice.label)));
            rows.push((DIM, format!("{DETAIL}{}", choice.detail)));
        }
        rows.extend([blank(), (DIM, self.help.clone())]);
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

    pub(super) fn screen(&self, pane: u64, cols: u16) -> Vec<Row> {
        self.questions
            .iter()
            .find(|question| question.pane == pane)
            .map(|question| question.rows(cols))
            .unwrap_or_default()
    }
}

fn blank() -> Row {
    (PLAIN, String::new())
}

#[cfg(test)]
mod tests;
