use rstest::rstest;
use serde_json::{Map, Value, json};

use super::*;
use crate::cli::surface::fixtures::ASK_ONE;
use crate::follow::fake::{BOOT, Mac, Said, Watched, connected, exits, refused, session};

const TAB: [&str; 8] = ["surface", "--tab", "3", "--cols", "80", "--rows", "24", "--patches"];
const SIZE: Size = Size { cols: 80, rows: 24 };
const RED: &str = "941: ❯ 1. Red (5)";

fn on_screen(edit: impl FnOnce(&mut Value)) -> Said {
    let mut frame: Value = serde_json::from_str(ASK_ONE).expect("the question fixture");
    edit(&mut frame);
    Said::Line(frame.to_string())
}

fn asked() -> Said {
    on_screen(|_| {})
}

fn row(text: &str) -> Value {
    json!([{ "style": 0, "cells": text.chars().map(String::from).collect::<Vec<_>>() }])
}

fn patched(rows: &[(u16, &str)]) -> Said {
    let lines: Map<String, Value> = rows.iter().map(|(at, text)| (at.to_string(), row(text))).collect();
    on_screen(|patch| {
        (patch["base"], patch["revision"]) = (1.into(), 2.into());
        patch["panes"] = json!([{ "pane": 941, "lines": lines }]);
    })
}

fn beside_a_shell() -> Said {
    on_screen(|frame| {
        frame["panes"][0]["active"] = false.into();
        let shell = json!({ "pane": 950, "active": true, "size": { "cols": 80, "rows": 1 }, "lines": [row("$ ls")] });
        frame["panes"].as_array_mut().expect("the panes of the frame").push(shell);
    })
}

fn described(question: &Question) -> String {
    let texts: Vec<String> =
        question.rows.iter().map(|line| line.iter().map(|run| run.text.as_str()).collect()).collect();
    let shown = texts.iter().find(|text| text.starts_with('❯')).or(texts.last());
    format!("{}: {} ({})", question.pane, shown.map_or("", String::as_str), question.options.len())
}

struct Read(Watched<Asked>);

impl Read {
    async fn after_a_minute(mac: Mac) -> Self {
        Self::of_pane(941, mac).await
    }

    async fn of_pane(pane: u64, mac: Mac) -> Self {
        let asking = Asking { tab: 3, pane, size: SIZE };
        Self(Watched::after_a_minute(mac, move |store, sink| asking.run(store, sink)).await)
    }

    fn seen(&mut self) -> Vec<(u64, String)> {
        self.0.seen(|asked| match asked {
            Asked::Question(question) => described(&question),
            Asked::Failed(_) => "failed".to_owned(),
            Asked::Closed => "closed".to_owned(),
        })
    }

    fn chosen(&mut self) -> Vec<(u64, Option<u8>)> {
        self.0.seen(|asked| match asked {
            Asked::Question(question) => question
                .card
                .and_then(|card| card.options.iter().find(|option| option.selected).map(|option| option.number)),
            Asked::Failed(_) | Asked::Closed => None,
        })
    }

    fn launched(&mut self) -> Vec<String> {
        self.0.launched(&["surface", "view "])
    }
}

fn reading(surfaces: Vec<Vec<Said>>) -> Mac {
    Mac::of(&TAB, connected(), surfaces)
}

mod the_surface {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn is_only_read_at_the_size_asked_without_claiming_the_tab() {
        let mut read = Read::after_a_minute(reading(vec![vec![asked()]])).await;
        assert_eq!(read.launched(), [TAB.join(" ")]);
    }

    #[tokio::test(start_paused = true)]
    async fn opens_again_when_it_ends_with_the_tab_still_there() {
        let surfaces = vec![vec![asked(), Said::After(5), exits()], vec![asked()]];
        let mut read = Read::after_a_minute(reading(surfaces)).await;
        assert_eq!(read.seen(), [(0, RED.to_owned()), (6, RED.to_owned())]);
        assert_eq!(read.launched().len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_tab_closed_and_stops_once_the_session_drops_it() {
        let sessions = vec![vec![session(BOOT, 7, true), Said::After(5), session(BOOT, 8, false)]];
        let surfaces = vec![vec![asked(), Said::After(5), exits()], vec![asked()]];
        let mut read = Read::after_a_minute(Mac::of(&TAB, sessions, surfaces)).await;
        assert_eq!(read.seen(), [(0, RED.to_owned()), (6, "closed".to_owned())]);
        assert!(read.0.running.is_finished());
    }

    #[tokio::test(start_paused = true)]
    async fn tells_once_that_it_keeps_failing() {
        let mut read = Read::after_a_minute(reading(vec![vec![refused("no tab 3")]])).await;
        assert_eq!(read.seen(), [(0, "failed".to_owned())]);
    }
}

mod the_question {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn is_said_with_its_rows_and_options_once_the_frame_shows_it() {
        let mut read = Read::after_a_minute(reading(vec![vec![asked()]])).await;
        assert_eq!(read.seen(), [(0, RED.to_owned())]);
    }

    #[tokio::test(start_paused = true)]
    async fn is_said_again_when_a_patch_moves_the_selection() {
        let moved = patched(&[(13, "  1. Red"), (15, "❯ 2. Green")]);
        let mut read = Read::after_a_minute(reading(vec![vec![asked(), Said::After(1), moved]])).await;
        assert_eq!(read.seen(), [(0, RED.to_owned()), (1, "941: ❯ 2. Green (5)".to_owned())]);
    }

    #[tokio::test(start_paused = true)]
    async fn carries_its_card_with_the_chosen_option_as_it_moves() {
        let moved = patched(&[(13, "  1. Red"), (15, "❯ 2. Green")]);
        let mut read = Read::after_a_minute(reading(vec![vec![asked(), Said::After(1), moved]])).await;
        assert_eq!(read.chosen(), [(0, Some(1)), (1, Some(2))]);
    }

    #[tokio::test(start_paused = true)]
    async fn has_no_card_in_a_pane_without_options() {
        let mut read = Read::of_pane(950, reading(vec![vec![beside_a_shell()]])).await;
        assert_eq!(read.chosen(), [(0, None)]);
    }

    #[tokio::test(start_paused = true)]
    async fn is_said_again_after_a_line_that_is_not_a_surface() {
        let surface = vec![asked(), Said::After(1), Said::Line("not json".to_owned()), Said::After(1), asked()];
        let mut read = Read::after_a_minute(reading(vec![surface])).await;
        assert_eq!(read.seen(), [(0, RED.to_owned()), (1, "failed".to_owned()), (2, RED.to_owned())]);
    }

    #[tokio::test(start_paused = true)]
    async fn is_not_said_again_for_a_patch_above_it() {
        let above = patched(&[(2, "Opus 5.5 with low effort")]);
        let mut read = Read::after_a_minute(reading(vec![vec![asked(), Said::After(1), above]])).await;
        assert_eq!(read.seen(), [(0, RED.to_owned())]);
    }

    #[rstest]
    #[case::of_the_agent(941, RED)]
    #[case::of_the_shell_beside_it(950, "950: $ ls (0)")]
    #[case::of_no_other_pane_when_the_one_asked_is_not_there(960, "960:  (0)")]
    #[tokio::test(start_paused = true)]
    async fn is_read_from_the_pane_asked_in_a_split_tab(#[case] pane: u64, #[case] expected: &str) {
        let mut read = Read::of_pane(pane, reading(vec![vec![beside_a_shell()]])).await;
        assert_eq!(read.seen(), [(0, expected.to_owned())]);
    }
}
