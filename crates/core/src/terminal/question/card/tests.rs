use rstest::rstest;

use super::*;
use crate::cli::surface::fixtures::{ASK_ONE, PERMISSION, WORKING};
use crate::terminal::question::cropped;
use crate::terminal::question::tests::{captured, line};

const COLS: u16 = 8;

fn option(number: u8, label: &str, detail: Option<&str>, selected: bool) -> CardOption {
    CardOption { number, label: label.to_owned(), detail: detail.map(str::to_owned), selected }
}

fn read(rows: &[&str]) -> Option<Vec<CardOption>> {
    let lines: Vec<Line> = rows.iter().map(|row| line(row)).collect();
    card(&lines, COLS).map(|card| card.options)
}

fn of(json: &str) -> Option<Card> {
    let pane = captured(json);
    card(cropped(&pane), pane.size.cols)
}

mod options {
    use super::*;

    #[rstest]
    #[case::without_details(&["1. Pear", "2. Plum"], vec![option(1, "Pear", None, false), option(2, "Plum", None, false)])]
    #[case::with_the_chosen_one(&[" ❯ 1. Pear", "   2. Plum"], vec![option(1, "Pear", None, true), option(2, "Plum", None, false)])]
    #[case::with_a_detail_under_one(&["1. Pear", "   Sweet", "2. Plum"], vec![option(1, "Pear", Some("Sweet"), false), option(2, "Plum", None, false)])]
    #[case::with_a_detail_on_two_rows(&["1. Pear", "   Sweet", "   and soft", "2. Plum"], vec![option(1, "Pear", Some("Sweet and soft"), false), option(2, "Plum", None, false)])]
    #[case::with_a_detail_under_the_last(&["1. Pear", "2. Plum", "   Dark"], vec![option(1, "Pear", None, false), option(2, "Plum", Some("Dark"), false)])]
    #[case::with_a_blank_row_inside_a_detail(&["1. Pear", "   Sweet", "", "   and soft", "2. Plum"], vec![option(1, "Pear", Some("Sweet and soft"), false), option(2, "Plum", None, false)])]
    #[case::with_a_rule_between_them(&["1. Pear", "────────", "2. Plum"], vec![option(1, "Pear", None, false), option(2, "Plum", None, false)])]
    #[case::with_rows_after_the_last(&["1. Pear", "2. Plum", "", "Esc to cancel"], vec![option(1, "Pear", None, false), option(2, "Plum", None, false)])]
    fn give_each_number_its_label_detail_and_mark(#[case] rows: &[&str], #[case] expected: Vec<CardOption>) {
        assert_eq!(read(rows), Some(expected));
    }

    #[rstest]
    #[case::with_one_option(&["Pick a fruit", "1. Pear"])]
    #[case::without_rows(&[])]
    #[case::with_a_row_between_options_not_indented_past_the_number(&["  1. Pear", "  Sweet", "  2. Plum"])]
    #[case::with_a_row_left_of_the_number(&["   1. Pear", " Sweet", "   2. Plum"])]
    #[case::with_a_short_rule_between_options(&["1. Pear", "───", "2. Plum"])]
    #[case::with_a_number_repeated_between_options(&["1. Pear", "2. Old", "2. Plum"])]
    fn are_none_when_a_row_does_not_fit(#[case] rows: &[&str]) {
        assert_eq!(read(rows), None);
    }
}

mod captures {
    use super::*;

    #[test]
    fn read_the_answers_to_a_question() {
        let colours = vec![
            option(1, "Red", Some("A warm, bold colour associated with energy and passion."), true),
            option(2, "Green", Some("A natural, calming colour associated with growth and balance."), false),
            option(3, "Blue", Some("A cool, steady colour associated with calm and trust."), false),
            option(4, "Type something.", None, false),
            option(5, "Chat about this", None, false),
        ];
        assert_eq!(of(ASK_ONE).map(|card| card.options), Some(colours));
    }

    #[test]
    fn read_the_answers_to_a_permission() {
        let answers = vec![
            option(1, "Yes", None, true),
            option(2, "Yes, and always allow access to", Some("/Users/dev/web-shop from this project"), false),
            option(3, "Yes, and switch to auto mode · auto mode handles these prompts for you", None, false),
            option(4, "No", None, false),
        ];
        assert_eq!(of(PERMISSION).map(|card| card.options), Some(answers));
    }

    #[test]
    fn read_nothing_while_the_agent_works() {
        assert_eq!(of(WORKING), None);
    }
}

mod serialized {
    use super::*;

    #[test]
    fn tags_each_block_with_its_kind() {
        let blocks = [Block::Text("Run it?".to_owned()), Block::Code("date".to_owned())];
        let expected = serde_json::json!([{ "kind": "text", "text": "Run it?" }, { "kind": "code", "text": "date" }]);
        assert_eq!(serde_json::to_value(blocks).expect("blocks as JSON"), expected);
    }
}
