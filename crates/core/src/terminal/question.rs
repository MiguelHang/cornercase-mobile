use serde::Serialize;

use super::grid::{Line, PaneGrid};

mod card;

pub use card::{Block, Card, CardOption, Tab, card};

pub const QUESTION_ROWS: usize = 24;

const MARKS: &str = "│┃|❯›>▶●○◯";

const CHOSEN: &str = "❯›>▶";

const RULE: [char; 12] = ['─', '━', '═', '╌', '╭', '╮', '╰', '╯', '┌', '┐', '└', '┘'];

const FENCE: char = '╌';

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Choice {
    pub number: u8,
    pub label: String,
}

#[must_use]
pub fn cropped(pane: &PaneGrid) -> &[Line] {
    let end = pane.lines.iter().rposition(|line| !is_blank(line)).map_or(0, |last| last + 1);
    let shown = pane.lines.get(..end).unwrap_or_default();
    let top = end.saturating_sub(QUESTION_ROWS);
    let start = opened(shown, top, pane.size.cols).unwrap_or_else(|| highest(shown, top, pane.size.cols));
    let asked = shown.get(start..).unwrap_or_default();
    let first = asked.iter().position(|line| !is_blank(line)).unwrap_or(asked.len());
    asked.get(first..).unwrap_or_default()
}

fn opened(shown: &[Line], top: usize, cols: u16) -> Option<usize> {
    let listed = listed(shown);
    let (first, last) = (listed.first()?, listed.last()?);
    if last.row < top {
        return None;
    }
    let above = shown.get(..first.row)?;
    above.iter().rposition(|line| is_rule(line, cols) && !chars(line).any(|c| c == FENCE))
}

fn highest(shown: &[Line], top: usize, cols: u16) -> usize {
    let window = shown.get(top..).unwrap_or_default();
    top + window.iter().position(|line| is_rule(line, cols)).unwrap_or(0)
}

#[must_use]
pub fn options(rows: &[Line]) -> Vec<Choice> {
    listed(rows).into_iter().map(|numbered| numbered.choice).collect()
}

struct Numbered {
    row: usize,
    column: usize,
    selected: bool,
    choice: Choice,
}

fn listed(rows: &[Line]) -> Vec<Numbered> {
    let mut found: Vec<Numbered> = Vec::new();
    for (row, (column, selected, choice)) in
        rows.iter().enumerate().rev().filter_map(|(row, line)| Some((row, numbered_at(line)?)))
    {
        if found.iter().any(|seen| seen.choice.number == choice.number) {
            continue;
        }
        let first = choice.number == 1;
        found.push(Numbered { row, column, selected, choice });
        if first {
            break;
        }
    }
    if found.len() < 2 {
        return Vec::new();
    }
    found.reverse();
    found
}

fn numbered_at(line: &Line) -> Option<(usize, bool, Choice)> {
    let text: String = chars(line).collect();
    let rest = text.trim_start_matches(is_padding);
    let (lead, _) = text.split_at_checked(text.len() - rest.len())?;
    let choice = choice(rest.trim_end_matches(is_padding))?;
    Some((lead.chars().count(), lead.contains(|c: char| CHOSEN.contains(c)), choice))
}

fn choice(text: &str) -> Option<Choice> {
    let mut rest = text.chars();
    let (Some(digit), Some('.')) = (rest.next(), rest.next()) else { return None };
    let number = u8::try_from(digit.to_digit(10)?).ok().filter(|number| (1..=9).contains(number))?;
    let label = rest.as_str().trim();
    (!label.is_empty()).then(|| Choice { number, label: label.to_owned() })
}

fn is_padding(c: char) -> bool {
    c.is_whitespace() || MARKS.contains(c)
}

fn chars(line: &Line) -> impl Iterator<Item = char> + '_ {
    line.iter().flat_map(|run| run.text.chars())
}

fn is_blank(line: &Line) -> bool {
    chars(line).all(char::is_whitespace)
}

fn is_rule(line: &Line, cols: u16) -> bool {
    let drawn = chars(line).filter(|c| RULE.contains(c)).count();
    drawn * 2 >= usize::from(cols.max(1)) && chars(line).all(|c| c == ' ' || RULE.contains(&c))
}

#[cfg(test)]
mod tests;
