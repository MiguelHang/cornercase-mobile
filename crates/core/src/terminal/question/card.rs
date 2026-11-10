use serde::Serialize;

use super::{Numbered, chars, is_blank, is_rule, listed};
use crate::cli::surface::Colour;
use crate::terminal::grid::Line;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Card {
    pub tabs: Vec<Tab>,
    #[cfg_attr(feature = "specta", specta(type = Option<u32>))]
    pub open: Option<usize>,
    pub blocks: Vec<Block>,
    pub options: Vec<CardOption>,
    pub help: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Tab {
    pub label: String,
    pub answered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", content = "text", rename_all = "lowercase")]
pub enum Block {
    Text(String),
    Code(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CardOption {
    pub number: u8,
    pub label: String,
    pub detail: Option<String>,
    pub selected: bool,
}

const TICKS: &str = "☐✔☒✓";

const ANSWERED: &str = "✔☒✓";

const FENCE: char = '╌';

enum Below {
    Skipped,
    Detail(String),
    Other,
}

struct Chip {
    tab: Tab,
    bg: Option<Colour>,
}

struct Blocks {
    cols: u16,
    done: Vec<Block>,
    lines: Vec<String>,
    code: Option<Vec<String>>,
}

#[must_use]
pub fn card(rows: &[Line], cols: u16) -> Option<Card> {
    let listed = listed(rows);
    let (first, last) = (listed.first()?, listed.last()?);
    let foot = footer(rows, last, cols);
    let options = answers(rows, &listed, foot, cols)?;
    let head = rows.get(..first.row)?;
    let tabbed = head.iter().enumerate().find_map(|(row, line)| Some((row, chips(line)?)));
    let open = tabbed.as_ref().and_then(|(_, chips)| open(chips));
    let tabs = tabbed.as_ref().map(|(_, chips)| chips.iter().map(|chip| chip.tab.clone()).collect());
    let blocks = blocks(head, tabbed.map(|(row, _)| row), cols)?;
    let help = help(rows.get(foot..)?, cols)?;
    Some(Card { tabs: tabs.unwrap_or_default(), open, blocks, options, help })
}

fn footer(rows: &[Line], last: &Numbered, cols: u16) -> usize {
    let mut under = rows.iter().enumerate().skip(last.row + 1);
    under.find(|(_, line)| matches!(below(line, last.column, cols), Below::Other)).map_or(rows.len(), |(row, _)| row)
}

fn answers(rows: &[Line], listed: &[Numbered], foot: usize, cols: u16) -> Option<Vec<CardOption>> {
    let ends = listed.iter().skip(1).map(|next| next.row).chain([foot]);
    listed.iter().zip(ends).map(|(numbered, end)| answer(rows, numbered, end, cols)).collect()
}

fn answer(rows: &[Line], numbered: &Numbered, end: usize, cols: u16) -> Option<CardOption> {
    let mut detail: Vec<String> = Vec::new();
    for line in rows.get(numbered.row + 1..end)? {
        match below(line, numbered.column, cols) {
            Below::Skipped => {}
            Below::Detail(text) => detail.push(text),
            Below::Other => return None,
        }
    }
    Some(CardOption {
        number: numbered.choice.number,
        label: numbered.choice.label.clone(),
        detail: (!detail.is_empty()).then(|| detail.join(" ")),
        selected: numbered.selected,
    })
}

fn below(line: &Line, column: usize, cols: u16) -> Below {
    if is_blank(line) || is_rule(line, cols) {
        return Below::Skipped;
    }
    if chars(line).take_while(|c| c.is_whitespace()).count() <= column {
        return Below::Other;
    }
    Below::Detail(trimmed(line))
}

fn chips(line: &Line) -> Option<Vec<Chip>> {
    let mut chips: Vec<Chip> = Vec::new();
    for (c, bg) in cells(line).filter(|(c, _)| !matches!(c, '←' | '→')) {
        if TICKS.contains(c) {
            chips.push(Chip { tab: Tab { label: String::new(), answered: ANSWERED.contains(c) }, bg });
        } else if let Some(chip) = chips.last_mut() {
            chip.tab.label.push(c);
        } else if !c.is_whitespace() {
            return None;
        }
    }
    for chip in &mut chips {
        chip.tab.label = chip.tab.label.trim().to_owned();
    }
    (!chips.is_empty() && chips.iter().all(|chip| !chip.tab.label.is_empty())).then_some(chips)
}

fn open(chips: &[Chip]) -> Option<usize> {
    let own = |at: usize, chip: &Chip| chips.iter().enumerate().all(|(other, them)| other == at || them.bg != chip.bg);
    let mut apart = chips.iter().enumerate().filter(|(at, chip)| chip.bg.is_some() && own(*at, chip));
    let (open, _) = apart.next()?;
    apart.next().is_none().then_some(open)
}

fn blocks(head: &[Line], tabbed: Option<usize>, cols: u16) -> Option<Vec<Block>> {
    let mut blocks = Blocks { cols, done: Vec::new(), lines: Vec::new(), code: None };
    for (_, line) in head.iter().enumerate().filter(|(row, _)| Some(*row) != tabbed) {
        blocks.read(line);
    }
    blocks.finish()
}

impl Blocks {
    fn read(&mut self, line: &Line) {
        let rule = is_rule(line, self.cols);
        let text: String = chars(line).collect();
        if rule && text.contains(FENCE) {
            self.fence();
        } else if let Some(code) = &mut self.code {
            code.push(text);
        } else if rule || is_blank(line) {
            self.paragraph();
        } else {
            self.lines.push(text);
        }
    }

    fn fence(&mut self) {
        if let Some(code) = self.code.take() {
            self.done.extend(unindented(&code).map(Block::Code));
        } else {
            self.paragraph();
            self.code = Some(Vec::new());
        }
    }

    fn paragraph(&mut self) {
        let lines = std::mem::take(&mut self.lines);
        if !lines.is_empty() {
            self.done.push(Block::Text(joined(&lines, self.cols)));
        }
    }

    fn finish(mut self) -> Option<Vec<Block>> {
        if self.code.is_some() {
            return None;
        }
        self.paragraph();
        Some(self.done)
    }
}

fn joined(rows: &[String], cols: u16) -> String {
    let mut text = rows.first().map(|row| row.trim().to_owned()).unwrap_or_default();
    for pair in rows.windows(2) {
        if let [row, next] = pair {
            text.push(if wrapped(row, next, cols) { ' ' } else { '\n' });
            text.push_str(next.trim());
        }
    }
    text
}

fn wrapped(row: &str, next: &str, cols: u16) -> bool {
    let row = row.trim_end();
    let indent = row.chars().take_while(|c| c.is_whitespace()).count();
    let word = next.split_whitespace().next().map_or(0, |word| word.chars().count());
    row.chars().count() + 1 + word > usize::from(cols).saturating_sub(indent)
}

fn unindented(rows: &[String]) -> Option<String> {
    let rows: Vec<&str> = rows.iter().map(|row| row.trim_end()).collect();
    let first = rows.iter().position(|row| !row.is_empty())?;
    let last = rows.iter().rposition(|row| !row.is_empty())?;
    let kept = rows.get(first..=last)?;
    let indent = |row: &str| row.chars().take_while(|c| c.is_whitespace()).count();
    let common = kept.iter().filter(|row| !row.is_empty()).map(|row| indent(row)).min()?;
    Some(kept.iter().map(|row| row.chars().skip(common).collect::<String>()).collect::<Vec<String>>().join("\n"))
}

fn help(rows: &[Line], cols: u16) -> Option<Vec<String>> {
    let help: Vec<String> = rows.iter().filter(|line| !is_blank(line) && !is_rule(line, cols)).map(trimmed).collect();
    (help.len() <= 2).then_some(help)
}

fn trimmed(line: &Line) -> String {
    chars(line).collect::<String>().trim().to_owned()
}

fn cells(line: &Line) -> impl Iterator<Item = (char, Option<Colour>)> + '_ {
    line.iter().flat_map(|run| run.text.chars().map(move |c| (c, run.style.bg)))
}

#[cfg(test)]
mod tests;
