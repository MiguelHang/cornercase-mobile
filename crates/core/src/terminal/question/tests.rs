use rstest::rstest;

use super::*;
use crate::cli::surface::fixtures::{ASK_ONE, ASK_TABS, PERMISSION, WORKING};
use crate::cli::surface::{Frame, Size, Style};
use crate::terminal::grid::{Grid, Run};

pub(super) fn line(text: &str) -> Line {
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

fn pairs(lines: &[Line]) -> Vec<(u8, String)> {
    options(lines).into_iter().map(|choice| (choice.number, choice.label)).collect()
}

fn listed(rows: &[&str]) -> Vec<(u8, String)> {
    pairs(&rows.iter().map(|row| line(row)).collect::<Vec<Line>>())
}

fn owned(expected: &[(u8, &str)]) -> Vec<(u8, String)> {
    expected.iter().map(|(number, label)| (*number, (*label).to_owned())).collect()
}

pub(super) fn captured(json: &str) -> PaneGrid {
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

mod options {
    use super::*;

    #[rstest]
    #[case::numbered_from_one(&["Pick a fruit", "1. Pear", "2. Plum", "3. Fig"], &[(1, "Pear"), (2, "Plum"), (3, "Fig")])]
    #[case::with_a_note_under_each(&["1. Pear", "   Sweet and soft", "2. Plum", "   Dark skin"], &[(1, "Pear"), (2, "Plum")])]
    #[case::with_the_chosen_one_marked(&[" ❯ 1. Pear", "   2. Plum"], &[(1, "Pear"), (2, "Plum")])]
    #[case::inside_a_box(&["│ › 1. Pear │", "│   2. Plum │"], &[(1, "Pear"), (2, "Plum")])]
    #[case::with_every_mark(&["┃▶ 1. Pear", "|● 2. Plum", ">○ 3. Fig", "◯ 4. Lime"], &[(1, "Pear"), (2, "Plum"), (3, "Fig"), (4, "Lime")])]
    #[case::with_spaces_around_the_label(&["1.   Pear  ", "2.Plum"], &[(1, "Pear"), (2, "Plum")])]
    #[case::up_to_nine(&["1. a", "2. b", "3. c", "4. d", "5. e", "6. f", "7. g", "8. h", "9. i"], &[(1, "a"), (2, "b"), (3, "c"), (4, "d"), (5, "e"), (6, "f"), (7, "g"), (8, "h"), (9, "i")])]
    #[case::without_the_first(&["2. Plum", "3. Fig"], &[(2, "Plum"), (3, "Fig")])]
    #[case::keeping_the_lowest_row_of_a_number(&["1. Pear", "2. Old", "2. Plum"], &[(1, "Pear"), (2, "Plum")])]
    #[case::stopping_at_the_first(&["1. Old", "2. Older", "1. Pear", "2. Plum"], &[(1, "Pear"), (2, "Plum")])]
    fn are_read_from_the_bottom_up(#[case] rows: &[&str], #[case] expected: &[(u8, &str)]) {
        assert_eq!(listed(rows), owned(expected));
    }

    #[rstest]
    #[case::with_one_option(&["Pick a fruit", "1. Pear"])]
    #[case::with_one_option_twice(&["1. Pear", "1. Pear"])]
    #[case::with_ten_and_more(&["10. Pear", "11. Plum"])]
    #[case::with_zero(&["0. Pear", "0. Plum"])]
    #[case::with_empty_labels(&["1.", "2.  ", "3. Fig"])]
    #[case::with_a_number_inside_the_text(&["Step 1. Pear", "and 2. Plum"])]
    #[case::with_a_number_without_a_dot(&["1) Pear", "2 Plum"])]
    #[case::with_letters(&["a. Pear", "b. Plum"])]
    #[case::without_rows(&[])]
    fn are_none_without_two_numbered_rows(#[case] rows: &[&str]) {
        assert_eq!(listed(rows), Vec::new());
    }
}

mod numbered_rows {
    use super::*;

    fn read(row: &str) -> Option<(usize, bool, u8, String)> {
        numbered_at(&line(row)).map(|(column, selected, choice)| (column, selected, choice.number, choice.label))
    }

    #[rstest]
    #[case::at_the_first_column("1. Pear", (0, false, 1, "Pear"))]
    #[case::after_spaces("   2. Plum", (3, false, 2, "Plum"))]
    #[case::chosen_with_a_chevron(" ❯ 1. Pear", (3, true, 1, "Pear"))]
    #[case::chosen_with_a_guillemet("│ › 3. Fig │", (4, true, 3, "Fig"))]
    #[case::chosen_with_a_triangle("▶ 4. Lime", (2, true, 4, "Lime"))]
    #[case::chosen_with_a_greater_than_sign(">5. Kiwi", (1, true, 5, "Kiwi"))]
    #[case::not_chosen_inside_a_box("│   2. Plum │", (4, false, 2, "Plum"))]
    #[case::not_chosen_with_a_circle("○ 6. Date", (2, false, 6, "Date"))]
    fn give_the_column_of_the_number_and_whether_it_is_chosen(
        #[case] row: &str,
        #[case] expected: (usize, bool, u8, &str),
    ) {
        let (column, selected, number, label) = expected;
        assert_eq!(read(row), Some((column, selected, number, label.to_owned())));
    }

    #[rstest]
    #[case::with_ten(" ❯ 10. Pear")]
    #[case::with_zero("0. Pear")]
    #[case::with_an_empty_label(" ❯ 1.  ")]
    #[case::with_text_before_the_number("Step 1. Pear")]
    #[case::with_only_marks(" ❯ │ ")]
    #[case::empty("")]
    fn are_none_without_an_option(#[case] row: &str) {
        assert_eq!(read(row), None);
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

    #[rstest]
    #[case::of_a_question(ASK_ONE)]
    #[case::of_two_questions(ASK_TABS)]
    fn list_the_answers_to_a_question(#[case] json: &str) {
        let colours = [(1, "Red"), (2, "Green"), (3, "Blue"), (4, "Type something."), (5, "Chat about this")];
        assert_eq!(pairs(cropped(&captured(json))), owned(&colours));
    }

    #[test]
    fn list_the_answers_to_a_permission() {
        let answers = [
            (1, "Yes"),
            (2, "Yes, and always allow access to"),
            (3, "Yes, and switch to auto mode · auto mode handles these prompts for you"),
            (4, "No"),
        ];
        assert_eq!(pairs(cropped(&captured(PERMISSION))), owned(&answers));
    }

    #[test]
    fn list_no_answers_while_the_agent_works() {
        assert_eq!(pairs(cropped(&captured(WORKING))), Vec::new());
    }

    #[test]
    fn keeps_the_question_and_its_options() {
        let rows = texts(cropped(&captured(PERMISSION)));
        let asked = [" Do you want to proceed?", " ❯ 1. Yes", "   4. No"];
        assert!(asked.iter().all(|row| rows.iter().any(|kept| kept == row)), "{rows:#?}");
    }
}
