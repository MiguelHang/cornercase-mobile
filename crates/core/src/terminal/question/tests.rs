use rstest::rstest;

use super::*;
use crate::cli::surface::fixtures::{ASK_ONE, ASK_TABS, PERMISSION, WORKING};
use crate::cli::surface::{Frame, Size, Style};
use crate::terminal::grid::{Grid, Run};

fn line(text: &str) -> Line {
    text.chars().map(|c| Run { text: c.to_string(), width: 1, style: Style::default(), link: None }).collect()
}

fn pane(cols: u16, rows: &[&str]) -> PaneGrid {
    let size = Size { cols, rows: u16::try_from(rows.len()).expect("a few rows") };
    PaneGrid { size, lines: rows.iter().map(|row| line(row)).collect(), ..PaneGrid::default() }
}

fn texts(lines: &[Line]) -> Vec<String> {
    lines.iter().map(|line| line.iter().map(|run| run.text.as_str()).collect()).collect()
}

fn numbered(count: usize) -> Vec<String> {
    (0..count).map(|row| format!("r{row}")).collect()
}

fn with_rule_on_top(count: usize) -> Vec<String> {
    let mut rows = numbered(count);
    rows[0] = "────".to_owned();
    rows
}

fn crops(cols: u16, rows: &[String]) -> Vec<String> {
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    texts(cropped(&pane(cols, &rows)))
}

fn captured(json: &str) -> PaneGrid {
    let mut grid = Grid::default();
    grid.apply(serde_json::from_str::<Frame>(json).expect("a surface frame"));
    grid.panes.into_iter().next().expect("the agent's pane")
}

mod window {
    use super::*;

    #[rstest]
    #[case::without_a_rule(&["", " ", "a", "", "b", "", "  "], &["a", "", "b"])]
    #[case::with_a_rule_shorter_than_half_the_pane(&["a", "───", "b"], &["a", "───", "b"])]
    #[case::with_a_rule_of_half_the_pane(&["a", "────", "b"], &["────", "b"])]
    #[case::with_two_rules(&["a", "────────", "b", "────────", "c"], &["────────", "b", "────────", "c"])]
    #[case::with_a_rule_that_holds_text(&["a", "── b ──", "c"], &["a", "── b ──", "c"])]
    #[case::with_a_box_top_among_spaces(&["a", " ╭──────╮ ", "│ b │", " ╰──────╯ "], &[" ╭──────╮ ", "│ b │", " ╰──────╯ "])]
    #[case::with_a_rule_of_every_kind(&["a", "━═╌┌┐└┘─", "b"], &["━═╌┌┐└┘─", "b"])]
    #[case::with_every_row_empty(&["", "  ", ""], &[])]
    #[case::without_rows(&[], &[])]
    fn starts_at_the_highest_rule(#[case] rows: &[&str], #[case] expected: &[&str]) {
        assert_eq!(texts(cropped(&pane(8, rows))), expected);
    }

    #[test]
    fn never_holds_more_rows_than_the_window() {
        assert_eq!(crops(8, &numbered(30)), numbered(30)[6..]);
    }

    #[test]
    fn counts_the_window_from_the_last_row_with_text() {
        let mut rows = numbered(30);
        rows.extend([String::new(), " ".to_owned()]);
        assert_eq!(crops(8, &rows), numbered(30)[6..]);
    }

    #[test]
    fn starts_at_a_rule_on_the_first_row_of_the_window() {
        let rows = with_rule_on_top(QUESTION_ROWS);
        assert_eq!(crops(8, &rows), rows);
    }

    #[test]
    fn ignores_a_rule_above_the_window() {
        let rows = with_rule_on_top(QUESTION_ROWS + 1);
        assert_eq!(crops(8, &rows), rows[1..]);
    }
}

mod captures {
    use super::*;

    #[rstest]
    #[case::of_a_question(ASK_ONE, 16, "Enter to select · ↑/↓ to navigate · Esc to cancel")]
    #[case::of_two_questions(ASK_TABS, 16, "Enter to select · Tab/Arrow keys to navigate · Esc to cancel")]
    #[case::of_a_permission(PERMISSION, 16, " Esc to cancel · Tab to amend")]
    #[case::of_an_agent_at_work(WORKING, 5, "  ⏸ manual mode on")]
    fn crop_from_the_rule_above_the_question(#[case] json: &str, #[case] count: usize, #[case] last: &str) {
        let pane = captured(json);
        let rows = texts(cropped(&pane));
        let rule = "─".repeat(usize::from(pane.size.cols));
        assert_eq!((rows.len(), rows.first(), rows.last().map(String::as_str)), (count, Some(&rule), Some(last)));
    }

    #[test]
    fn keeps_the_question_and_its_options() {
        let rows = texts(cropped(&captured(PERMISSION)));
        let asked = [" Do you want to proceed?", " ❯ 1. Yes", "   4. No"];
        assert!(asked.iter().all(|row| rows.iter().any(|kept| kept == row)), "{rows:#?}");
    }
}
