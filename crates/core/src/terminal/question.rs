use super::grid::{Line, PaneGrid};

pub const QUESTION_ROWS: usize = 24;

const RULE: [char; 12] = ['─', '━', '═', '╌', '╭', '╮', '╰', '╯', '┌', '┐', '└', '┘'];

#[must_use]
pub fn cropped(pane: &PaneGrid) -> &[Line] {
    let end = pane.lines.iter().rposition(|line| !is_blank(line)).map_or(0, |last| last + 1);
    let window = pane.lines.get(end.saturating_sub(QUESTION_ROWS)..end).unwrap_or_default();
    let ruled = window.iter().position(|line| is_rule(line, pane.size.cols)).unwrap_or(0);
    let asked = window.get(ruled..).unwrap_or_default();
    let first = asked.iter().position(|line| !is_blank(line)).unwrap_or(asked.len());
    asked.get(first..).unwrap_or_default()
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
