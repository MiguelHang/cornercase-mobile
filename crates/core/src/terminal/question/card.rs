use serde::Serialize;

use super::{Numbered, chars, is_blank, is_rule, listed};
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

enum Below {
    Skipped,
    Detail(String),
    Other,
}

#[must_use]
pub fn card(rows: &[Line], cols: u16) -> Option<Card> {
    let listed = listed(rows);
    if listed.is_empty() {
        return None;
    }
    let options = answers(rows, &listed, cols)?;
    Some(Card { tabs: Vec::new(), open: None, blocks: Vec::new(), options, help: Vec::new() })
}

fn answers(rows: &[Line], listed: &[Numbered], cols: u16) -> Option<Vec<CardOption>> {
    let ends = listed.iter().skip(1).map(|next| Some(next.row)).chain([None]);
    listed.iter().zip(ends).map(|(numbered, end)| answer(rows, numbered, end, cols)).collect()
}

fn answer(rows: &[Line], numbered: &Numbered, end: Option<usize>, cols: u16) -> Option<CardOption> {
    let under = rows.get(numbered.row + 1..end.unwrap_or(rows.len()))?;
    let mut detail: Vec<String> = Vec::new();
    for line in under {
        match below(line, numbered.column, cols) {
            Below::Skipped => {}
            Below::Detail(text) => detail.push(text),
            Below::Other if end.is_none() => break,
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
    Below::Detail(chars(line).collect::<String>().trim().to_owned())
}

#[cfg(test)]
mod tests;
