use serde::Serialize;

use super::grid::{Line, PaneGrid};

pub const QUESTION_ROWS: usize = 24;

const MARKS: &str = "│┃|❯›>▶●○◯";

const CHOSEN: &str = "❯›>▶";

const RULE: [char; 12] = ['─', '━', '═', '╌', '╭', '╮', '╰', '╯', '┌', '┐', '└', '┘'];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Choice {
    pub number: u8,
    pub label: String,
}

#[must_use]
pub fn cropped(pane: &PaneGrid) -> &[Line] {
    let end = pane.lines.iter().rposition(|line| !is_blank(line)).map_or(0, |last| last + 1);
    let window = pane.lines.get(end.saturating_sub(QUESTION_ROWS)..end).unwrap_or_default();
    let ruled = window.iter().position(|line| is_rule(line, pane.size.cols)).unwrap_or(0);
    let asked = window.get(ruled..).unwrap_or_default();
    let first = asked.iter().position(|line| !is_blank(line)).unwrap_or(asked.len());
    asked.get(first..).unwrap_or_default()
}

#[must_use]
pub fn options(rows: &[Line]) -> Vec<Choice> {
    let mut found: Vec<Choice> = Vec::new();
    for choice in rows.iter().rev().filter_map(numbered) {
        if found.iter().any(|seen| seen.number == choice.number) {
            continue;
        }
        let first = choice.number == 1;
        found.push(choice);
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

fn numbered(line: &Line) -> Option<Choice> {
    numbered_at(line).map(|(_, _, choice)| choice)
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
