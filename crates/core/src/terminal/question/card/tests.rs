use rstest::rstest;

use super::*;
use crate::cli::surface::Style;
use crate::cli::surface::fixtures::{ASK_LONG, ASK_ONE, ASK_REVIEW, ASK_TABS, PERMISSION, PERMISSION_GUTTER, WORKING};
use crate::terminal::grid::Run;
use crate::terminal::question::cropped;
use crate::terminal::question::tests::{captured, line};

const COLS: u16 = 8;

const PLAIN: [&str; 2] = ["1. Pear", "2. Plum"];

fn option(number: u8, label: &str, detail: Option<&str>, selected: bool) -> CardOption {
    CardOption { number, label: label.to_owned(), detail: detail.map(str::to_owned), selected }
}

fn tab(label: &str, answered: bool) -> Tab {
    Tab { label: label.to_owned(), answered }
}

fn text(text: &str) -> Block {
    Block::Text(text.to_owned())
}

fn parsed(lines: &[Line]) -> Option<Card> {
    card(lines, COLS)
}

fn plain(rows: &[&str]) -> Vec<Line> {
    rows.iter().map(|row| line(row)).collect()
}

fn read(rows: &[&str]) -> Option<Vec<CardOption>> {
    parsed(&plain(rows)).map(|card| card.options)
}

fn above(rows: &[&str]) -> Option<Card> {
    parsed(&plain(&[rows, &PLAIN].concat()))
}

fn painted(parts: &[(&str, Option<u8>)]) -> Line {
    let cell = |c: char, bg: Option<u8>| Run {
        text: c.to_string(),
        width: 1,
        style: Style { bg: bg.map(Colour::Index), ..Style::default() },
        link: None,
    };
    parts.iter().flat_map(|(text, bg)| text.chars().map(move |c| cell(c, *bg))).collect()
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

mod tabs {
    use super::*;

    #[rstest]
    #[case::with_the_open_one_painted(&[(" ☐ Pear ", Some(153)), (" ✔ Plum", None)], &[("Pear", false), ("Plum", true)], Some(0))]
    #[case::without_the_arrows(&[("←  ", None), ("☐ Pear", None), ("  ☒ Fig", Some(153)), ("  →", None)], &[("Pear", false), ("Fig", true)], Some(1))]
    #[case::with_a_lone_painted_one(&[(" ☐ Pear ", Some(153))], &[("Pear", false)], Some(0))]
    #[case::with_the_open_one_on_a_painted_row(&[("☐ Pear ", Some(1)), ("☐ Fig ", Some(2)), ("✓ Plum", Some(2))], &[("Pear", false), ("Fig", false), ("Plum", true)], Some(0))]
    #[case::without_a_background_of_their_own(&[("☐ Pear ☐ Plum", None)], &[("Pear", false), ("Plum", false)], None)]
    #[case::with_every_one_painted_alike(&[("☐ Pear ☐ Plum", Some(153))], &[("Pear", false), ("Plum", false)], None)]
    #[case::with_two_painted_apart(&[("☐ Pear ", Some(1)), ("☐ Fig ", Some(2)), ("☐ Plum", None)], &[("Pear", false), ("Fig", false), ("Plum", false)], None)]
    fn are_read_from_the_row_above_the_options(
        #[case] chips: &[(&str, Option<u8>)],
        #[case] expected: &[(&str, bool)],
        #[case] open: Option<usize>,
    ) {
        let lines = [vec![painted(chips)], plain(&PLAIN)].concat();
        let card = parsed(&lines).expect("a card");
        let tabs: Vec<Tab> = expected.iter().map(|(label, answered)| tab(label, *answered)).collect();
        assert_eq!((card.tabs, card.open, card.blocks), (tabs, open, Vec::new()));
    }

    #[rstest]
    #[case::with_text_before_the_mark(&["Pick ☐ one"], "Pick ☐ one")]
    #[case::with_a_mark_and_no_label(&["☐ Pear ✔"], "☐ Pear ✔")]
    #[case::under_the_first_row_with_text(&["Ready?", "✓ Tests"], "Ready?\n✓ Tests")]
    fn are_none_when_a_row_is_not_made_of_them(#[case] rows: &[&str], #[case] said: &str) {
        let card = above(rows).expect("a card");
        assert_eq!((card.tabs, card.open, card.blocks), (Vec::new(), None, vec![text(said)]));
    }
}

mod blocks {
    use super::*;

    #[rstest]
    #[case::with_one_row(&["Pick one"], vec![text("Pick one")])]
    #[case::with_a_row_wrapped_to_the_pane(&["Pick a", "fruit"], vec![text("Pick a fruit")])]
    #[case::with_a_row_that_ends_short(&["Hi", "there"], vec![text("Hi\nthere")])]
    #[case::with_a_word_that_fits_only_without_the_space(&["abc", "defgh"], vec![text("abc defgh")])]
    #[case::with_an_indented_row_wrapped_inside_its_margin(&["  Hi", "  there"], vec![text("Hi there")])]
    #[case::with_a_blank_row_between_paragraphs(&["Pick", "", "", "Now"], vec![text("Pick"), text("Now")])]
    #[case::with_a_rule_dropped(&["────────", "Pick", "────────", "Now"], vec![text("Pick"), text("Now")])]
    #[case::with_code_between_two_dashed_rules(&["Run", "╌╌╌╌╌╌╌╌", "", "  ls", "    -a", "", "╌╌╌╌╌╌╌╌", "Ok?"], vec![text("Run"), Block::Code("ls\n  -a".to_owned()), text("Ok?")])]
    #[case::with_a_blank_row_inside_the_code(&["╌╌╌╌╌╌╌╌", " ls", "", " pwd", "╌╌╌╌╌╌╌╌"], vec![Block::Code("ls\n\npwd".to_owned())])]
    #[case::with_a_rule_inside_the_code(&["╌╌╌╌╌╌╌╌", "────────", "╌╌╌╌╌╌╌╌"], vec![Block::Code("────────".to_owned())])]
    #[case::with_a_bar_down_the_code(&["╌╌╌╌╌╌╌╌", " │ ls", " │   -a", "╌╌╌╌╌╌╌╌"], vec![Block::Code("ls\n  -a".to_owned())])]
    #[case::with_a_bar_down_the_text(&["│ Pick a", "│ fruit"], vec![text("Pick a fruit")])]
    #[case::with_a_bar_counted_in_the_width_of_a_row(&["│ Pick", "│ ab"], vec![text("Pick ab")])]
    #[case::with_a_bar_down_a_text_that_ends_short(&["│ Hi", "│ yo"], vec![text("Hi\nyo")])]
    #[case::with_a_bar_on_some_text_rows_only(&["│ Hi", "Yo"], vec![text("Hi\nYo")])]
    #[case::with_a_bar_on_the_rows_of_a_wrapped_sentence_only(&["│ Pick a", "│ fig", "→ Fig"], vec![text("Pick a fig\n→ Fig")])]
    #[case::with_a_bar_without_a_space(&["│Hi", "Yo"], vec![text("Hi\nYo")])]
    #[case::with_a_bar_on_some_code_rows_only(&["╌╌╌╌╌╌╌╌", " │ ls", " pwd", "╌╌╌╌╌╌╌╌"], vec![Block::Code("│ ls\npwd".to_owned())])]
    #[case::with_nothing_between_two_dashed_rules(&["Run", "╌╌╌╌╌╌╌╌", "  ", "╌╌╌╌╌╌╌╌"], vec![text("Run")])]
    fn split_the_rows_above_the_options(#[case] rows: &[&str], #[case] expected: Vec<Block>) {
        assert_eq!(above(rows).map(|card| card.blocks), Some(expected));
    }

    #[test]
    fn are_none_when_a_dashed_rule_does_not_close() {
        assert_eq!(above(&["Run", "╌╌╌╌╌╌╌╌", "ls"]), None);
    }
}

mod help {
    use super::*;

    #[rstest]
    #[case::without_rows(&[], &[])]
    #[case::with_one_row(&["", "Esc to cancel"], &["Esc to cancel"])]
    #[case::with_two_rows_and_a_rule(&["────────", "Enter", "", "Esc"], &["Enter", "Esc"])]
    fn is_read_from_the_rows_after_the_options(#[case] rows: &[&str], #[case] expected: &[&str]) {
        let help = parsed(&plain(&[&PLAIN, rows].concat())).map(|card| card.help);
        assert_eq!(help, Some(expected.iter().map(|row| (*row).to_owned()).collect()));
    }

    #[test]
    fn is_none_with_more_than_two_rows() {
        assert_eq!(parsed(&plain(&["1. Pear", "2. Plum", "Enter", "Tab", "Esc"])), None);
    }
}

mod captures {
    use super::*;

    fn colours() -> Vec<CardOption> {
        vec![
            option(1, "Red", Some("A warm, bold colour associated with energy and passion."), true),
            option(2, "Green", Some("A natural, calming colour associated with growth and balance."), false),
            option(3, "Blue", Some("A cool, steady colour associated with calm and trust."), false),
            option(4, "Type something.", None, false),
            option(5, "Chat about this", None, false),
        ]
    }

    #[test]
    fn read_a_question() {
        let expected = Card {
            tabs: vec![tab("Colour", false)],
            open: Some(0),
            blocks: vec![text("Which colour do you prefer?")],
            options: colours(),
            help: vec!["Enter to select · ↑/↓ to navigate · Esc to cancel".to_owned()],
        };
        assert_eq!(of(ASK_ONE), Some(expected));
    }

    #[test]
    fn read_a_question_among_several() {
        let expected = Card {
            tabs: vec![tab("Colour", false), tab("Size", false), tab("Submit", true)],
            open: Some(0),
            blocks: vec![text("Which colour do you prefer?")],
            options: colours(),
            help: vec!["Enter to select · Tab/Arrow keys to navigate · Esc to cancel".to_owned()],
        };
        assert_eq!(of(ASK_TABS), Some(expected));
    }

    #[test]
    fn read_a_permission() {
        let said = "Bash command\nTip: auto mode handles these prompts for you — choose \"switch to auto mode\" below\nWrite current date to later.txt";
        let expected = Card {
            tabs: Vec::new(),
            open: None,
            blocks: vec![text(said), Block::Code("date > later.txt".to_owned()), text("Do you want to proceed?")],
            options: vec![
                option(1, "Yes", None, true),
                option(2, "Yes, and always allow access to", Some("/Users/dev/web-shop from this project"), false),
                option(3, "Yes, and switch to auto mode · auto mode handles these prompts for you", None, false),
                option(4, "No", None, false),
            ],
            help: vec!["Esc to cancel · Tab to amend".to_owned()],
        };
        assert_eq!(of(PERMISSION), Some(expected));
    }

    #[test]
    fn read_a_permission_with_a_bar_down_its_command() {
        let said = "Bash command\nTip: auto mode handles these prompts for you — choose \"switch to auto mode\" below\nWrite Greek letters to file and count words";
        let command = "printf 'alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma\\n' >\ngreek-letters-for-the-phone-check.txt && wc -w greek-letters-for-the-phone-check.txt";
        let folder = "Yes, and always allow access to /Users/dev/code/clients/a-folder-name-long-enough-to-reach-the-edge-of-the-pane-and-wrap-";
        let expected = Card {
            tabs: Vec::new(),
            open: None,
            blocks: vec![text(said), Block::Code(command.to_owned()), text("Do you want to proceed?")],
            options: vec![
                option(1, "Yes", None, true),
                option(2, folder, Some("keeps-going-on-this-row/clients/web-shop-app from this project"), false),
                option(3, "Yes, and switch to auto mode · auto mode handles these prompts for you", None, false),
                option(4, "No", None, false),
            ],
            help: vec!["Esc to cancel · Tab to amend".to_owned()],
        };
        assert_eq!(of(PERMISSION_GUTTER), Some(expected));
    }

    fn databases() -> Vec<CardOption> {
        vec![
            option(
                1,
                "PostgreSQL",
                Some(
                    "A mature open-source relational database with strong consistency and rich SQL features. It handles accounts, orders, and analytics queries well, runs on nearly every host, and has years of documented operational practice that a small team can rely on.",
                ),
                true,
            ),
            option(
                2,
                "MySQL",
                Some(
                    "A widely deployed relational database with a simple replication model and broad hosting support across providers. It suits straightforward transactional workloads, is easy to find help for, and works well with many frameworks. Fewer advanced SQL features are its main drawback for analytics.",
                ),
                false,
            ),
            option(
                3,
                "SQLite",
                Some(
                    "An embedded database stored in a single file, with zero servers to run. It is fast for a small team's read-heavy workloads and can be backed up by copying the file. Concurrent writes and horizontal scaling are limited, so growth may force a later migration.",
                ),
                false,
            ),
            option(
                4,
                "MongoDB",
                Some(
                    "A document database with flexible schemas that speeds early development when data shapes change often. It scales out through built-in sharding and offers managed cloud hosting. Relational joins for orders and reporting are awkward, so analytics usually need extra tooling or denormalized copies.",
                ),
                false,
            ),
            option(5, "Type something.", None, false),
            option(6, "Chat about this", None, false),
        ]
    }

    #[test]
    fn read_a_question_taller_than_the_window_with_a_bar_down_its_text() {
        let asked = "Our team has five engineers and a modest budget, and we must store user accounts, orders, and analytics data with reliable backups and simple operations. Which database should this small team choose for the project?";
        let expected = Card {
            tabs: vec![tab("DB", false), tab("Hosting", false), tab("On-call", false), tab("Submit", true)],
            open: Some(0),
            blocks: vec![text(asked)],
            options: databases(),
            help: vec!["Enter to select · Tab/Arrow keys to navigate · Esc to cancel".to_owned()],
        };
        assert_eq!(of(ASK_LONG), Some(expected));
    }

    #[test]
    fn read_the_review_of_several_answers_with_a_bar_down_each_question() {
        let summary = [
            "● Our small team is shipping a multi-tenant web shop next quarter, and every merchant's data must stay isolated. Which database approach should we commit to for tenant separation, scaling, and day-to-day operations, with only a few engineers available to maintain it?",
            "→ Shared tables with tenant_id",
            "● Our small team is shipping a multi-tenant web shop next quarter and needs a hosting setup that handles traffic spikes during sales without a dedicated platform group. Which hosting model should we choose, balancing cost, deployment speed, and how much infrastructure the team must own?",
            "→ Serverless or PaaS",
            "● Our small team is shipping a multi-tenant web shop next quarter, and a outage could hit many merchants at once. Since we cannot staff a full operations group, which on-call model should we adopt for handling production incidents and overnight alerts?",
            "→ Business hours only",
        ];
        let expected = Card {
            tabs: vec![tab("DB", true), tab("Hosting", true), tab("On-call", true), tab("Submit", true)],
            open: Some(3),
            blocks: vec![text("Review your answers"), text(&summary.join("\n")), text("Ready to submit your answers?")],
            options: vec![option(1, "Submit answers", None, true), option(2, "Cancel", None, false)],
            help: Vec::new(),
        };
        assert_eq!(of(ASK_REVIEW), Some(expected));
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
