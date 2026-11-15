use std::fmt::Debug;

use rstest::rstest;

use super::*;

const V1: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/history/v1.json"));
const SECRET_WORD: &str = "do-not-repeat-me";

fn you(id: u64, text: &str) -> HistoryEntry {
    HistoryEntry::You {
        id,
        text: text.to_owned(),
        chips: Vec::new(),
        state: SendState::Sent,
        reason: None,
        seen: 1_000 + id,
    }
}

fn agent(id: u64, text: &str) -> HistoryEntry {
    HistoryEntry::Agent { id, text: text.to_owned(), written: None, turn_over: Some(true), seen: 1_000 + id }
}

fn gap(id: u64) -> HistoryEntry {
    HistoryEntry::Gap { id, seen: 1_000 + id }
}

fn turns(count: u64) -> Vec<HistoryEntry> {
    (0..count).map(|id| if id % 2 == 0 { you(id, "ask") } else { agent(id, "answer") }).collect()
}

fn store_error<T: Debug>(result: Result<T, Error>) -> String {
    match result {
        Err(Error::Store(message)) => message,
        other => panic!("expected a store error, got {other:?}"),
    }
}

fn ids(entries: &[HistoryEntry]) -> Vec<(u64, &'static str)> {
    entries
        .iter()
        .map(|entry| match entry {
            HistoryEntry::You { id, .. } => (*id, "you"),
            HistoryEntry::Agent { id, .. } => (*id, "agent"),
            HistoryEntry::Gap { id, .. } => (*id, "gap"),
        })
        .collect()
}

fn budget_for(entries: &[HistoryEntry]) -> usize {
    entries.iter().map(|entry| serde_json::to_vec(entry).expect("measure the entry").len() + 1).sum()
}

mod capped {
    use super::*;

    #[test]
    fn keeps_a_history_within_both_limits_as_it_is() {
        let entries = turns(200);
        assert_eq!(capped(&entries, usize::MAX).expect("cap"), entries);
    }

    #[test]
    fn drops_the_oldest_entries_past_two_hundred_and_puts_a_gap_in_front() {
        let kept = capped(&turns(201), usize::MAX).expect("cap");
        assert_eq!(kept.len(), 200);
        assert_eq!(ids(&kept[..2]), [(1, "gap"), (2, "you")]);
        assert_eq!(kept.last(), turns(201).last());
    }

    #[test]
    fn gives_the_gap_the_place_of_the_last_entry_it_dropped() {
        let kept = capped(&turns(205), usize::MAX).expect("cap");
        assert_eq!(kept[0], HistoryEntry::Gap { id: 5, seen: 1_005 });
    }

    #[test]
    fn keeps_the_gap_already_in_front_instead_of_adding_a_second() {
        let mut entries = vec![you(0, "ask"), gap(1)];
        entries.extend((2..201).map(|id| agent(id, "answer")));
        let kept = capped(&entries, usize::MAX).expect("cap");
        assert_eq!(kept.len(), 200);
        assert_eq!(ids(&kept[..2]), [(1, "gap"), (2, "agent")]);
    }

    #[test]
    fn never_leaves_two_gaps_in_a_row_when_the_oldest_was_a_gap() {
        let mut entries = vec![gap(0)];
        entries.extend((1..201).map(|id| agent(id, "answer")));
        let kept = capped(&entries, usize::MAX).expect("cap");
        assert_eq!(ids(&kept[..2]), [(1, "gap"), (2, "agent")]);
        assert_eq!(kept.iter().filter(|entry| matches!(entry, HistoryEntry::Gap { .. })).count(), 1);
    }

    #[test]
    fn drops_the_oldest_entries_past_the_bytes_it_may_take() {
        let entries = [you(0, &"a".repeat(500)), agent(1, &"b".repeat(500)), you(2, "short"), agent(3, "reply")];
        let budget = budget_for(&[you(2, "short"), agent(3, "reply")]) + 100;
        let kept = capped(&entries, budget).expect("cap");
        assert_eq!(ids(&kept), [(1, "gap"), (2, "you"), (3, "agent")]);
    }

    #[test]
    fn drops_an_entry_when_the_whole_history_is_one_byte_over_the_budget() {
        let entries = turns(4);
        let kept = capped(&entries, budget_for(&entries) - 1).expect("cap");
        assert!(matches!(kept[0], HistoryEntry::Gap { .. }), "{kept:?}");
        assert!(kept.len() < entries.len() + 1);
    }

    #[test]
    fn counts_the_gap_it_adds_against_the_bytes_it_may_take() {
        let entries = [you(0, &"a".repeat(500)), agent(1, &"b".repeat(500)), you(2, "short"), agent(3, "reply")];
        let gap_size = size(&gap(0)).expect("measure the gap");
        let budget = budget_for(&[you(2, "short"), agent(3, "reply")]) + gap_size - 1;
        let kept = capped(&entries, budget).expect("cap");
        assert_eq!(ids(&kept), [(2, "gap"), (3, "agent")]);
    }

    #[test]
    fn adds_no_gap_in_front_of_a_gap_that_the_cut_reaches() {
        let mut entries = vec![you(0, "ask"), you(1, "ask"), gap(2)];
        entries.extend((3..201).map(|id| agent(id, "answer")));
        let kept = capped(&entries, usize::MAX).expect("cap");
        assert_eq!(kept.len(), 199);
        assert_eq!(ids(&kept[..2]), [(2, "gap"), (3, "agent")]);
    }

    #[test]
    fn leaves_only_a_gap_when_even_the_newest_entry_is_too_big() {
        let entries = [you(0, "ask"), agent(1, &"b".repeat(5_000))];
        let kept = capped(&entries, 1_000).expect("cap");
        assert_eq!(kept, [gap(1)]);
    }
}

mod written {
    use super::*;

    #[test]
    fn reads_back_what_it_wrote() {
        let entries = turns(4);
        let text = written("boot-1", &entries).expect("write");
        assert_eq!(parsed(&text).expect("read"), PaneFile { boot: "boot-1".to_owned(), entries });
    }

    #[test]
    fn writes_the_format_version() {
        let text = written("boot-1", &[]).expect("write");
        let value: serde_json::Value = serde_json::from_slice(&text).expect("JSON");
        assert_eq!(value, serde_json::json!({"version": 1, "boot": "boot-1", "entries": []}));
    }

    #[test]
    fn keeps_the_file_within_one_mebibyte() {
        let entries = (0..20).map(|id| agent(id, &"x".repeat(100_000))).collect::<Vec<_>>();
        let text = written("boot-1", &entries).expect("write");
        assert!(text.len() <= 1 << 20, "{} bytes", text.len());
        let kept = parsed(&text).expect("read").entries;
        assert_eq!(ids(&kept[..1]), [(9, "gap")]);
        assert_eq!(kept.last(), entries.last());
    }
}

mod parsed {
    use super::*;

    #[test]
    fn reads_the_version_one_file() {
        let file = parsed(V1.as_bytes()).expect("read");
        assert_eq!(file.boot, "1731250000-4242");
        let expected = [
            HistoryEntry::Gap { id: 0, seen: 1_731_250_001_000 },
            HistoryEntry::You {
                id: 1,
                text: "fix the login".to_owned(),
                chips: vec!["screenshot.png".to_owned()],
                state: SendState::Sent,
                reason: None,
                seen: 1_731_250_002_000,
            },
            HistoryEntry::Agent {
                id: 2,
                text: "Fixed it.".to_owned(),
                written: Some("2026-11-10T14:52:03Z".to_owned()),
                turn_over: Some(true),
                seen: 1_731_250_013_000,
            },
            HistoryEntry::You {
                id: 3,
                text: "now the tests".to_owned(),
                chips: Vec::new(),
                state: SendState::Failed,
                reason: Some("the connection to the Mac dropped before the command finished".to_owned()),
                seen: 1_731_250_020_000,
            },
        ];
        assert_eq!(file.entries, expected);
    }

    #[rstest]
    #[case::not_json(format!("{{\"version\": 1, {SECRET_WORD}"), "is not JSON")]
    #[case::cut_short(r#"{"version": 1, "boot": "b", "entries": ["#.to_owned(), "is not JSON")]
    #[case::unknown_version(
        format!(r#"{{"version": 2, "boot": "b", "entries": [{{"kind": "agent", "text": "{SECRET_WORD}"}}]}}"#),
        "the file's format version 2 is unknown"
    )]
    #[case::no_version(format!(r#"{{"boot": "{SECRET_WORD}", "entries": []}}"#), "does not hold the expected fields")]
    #[case::no_boot(r#"{"version": 1, "entries": []}"#.to_owned(), "does not hold the expected fields")]
    #[case::entry_without_its_text(
        format!(r#"{{"version": 1, "boot": "{SECRET_WORD}", "entries": [{{"kind": "agent", "id": 0, "seen": 1}}]}}"#),
        "does not hold the expected fields"
    )]
    #[case::unknown_kind(
        format!(r#"{{"version": 1, "boot": "b", "entries": [{{"kind": "{SECRET_WORD}", "id": 0, "seen": 1}}]}}"#),
        "does not hold the expected fields"
    )]
    #[case::unknown_state(
        format!(
            r#"{{"version": 1, "boot": "b", "entries": [{{"kind": "you", "id": 0, "text": "t", "chips": [], "state": "{SECRET_WORD}", "reason": null, "seen": 1}}]}}"#
        ),
        "does not hold the expected fields"
    )]
    fn refuses_a_file_it_cannot_read_without_quoting_it(#[case] text: String, #[case] reason: &str) {
        let message = store_error(parsed(text.as_bytes()));
        assert!(message.contains(reason), "{message}");
        assert!(!message.contains(SECRET_WORD), "{message}");
    }

    #[test]
    fn reads_an_entry_without_its_optional_fields() {
        let text = br#"{"version": 1, "boot": "b", "entries": [{"kind": "agent", "id": 4, "text": "t", "seen": 9}]}"#;
        let entries = parsed(text).expect("read").entries;
        assert_eq!(
            entries,
            [HistoryEntry::Agent { id: 4, text: "t".to_owned(), written: None, turn_over: None, seen: 9 }]
        );
    }
}
