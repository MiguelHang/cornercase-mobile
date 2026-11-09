use std::time::{Duration, Instant};

use super::*;

const PATCHES: u64 = 1_000;
const ROWS: u16 = 40;

fn cells(count: usize, seed: usize) -> Vec<String> {
    (0..count).map(|i| char::from(b'a' + u8::try_from((i + seed) % 26).unwrap_or(0)).to_string()).collect()
}

fn coloured(seed: usize) -> Value {
    json!([
        { "style": 1, "cells": cells(10, seed) },
        { "cells": cells(90, seed) },
        { "style": 2, "hyperlink": 0, "cells": cells(20, seed) }
    ])
}

fn screen() -> Value {
    let lines: Vec<_> = (0..usize::from(ROWS)).map(coloured).collect();
    json!({
        "boot": "b", "surface": 1, "revision": 0, "cols": 120, "rows": ROWS,
        "styles": [{}, { "bold": true }, { "underline": true }],
        "hyperlinks": [{ "uri": DOCS }],
        "panes": [{ "pane": 1, "active": true, "size": { "cols": 120, "rows": ROWS }, "lines": lines }]
    })
}

fn scrolled(revision: u64) -> String {
    let fresh = usize::try_from(revision % 2 + 1).unwrap_or(1);
    let last = usize::from(ROWS) - 1;
    let lines: serde_json::Map<String, Value> =
        (0..fresh).map(|i| ((last - i).to_string(), coloured(usize::try_from(revision).unwrap_or(0) + i))).collect();
    let wire = json!({
        "pane": 1,
        "cursor": { "x": 0, "y": last },
        "copy": { "from": fresh, "to": 0, "count": usize::from(ROWS) - fresh },
        "lines": lines
    });
    json!({
        "boot": "b", "surface": 1, "revision": revision + 1, "base": revision,
        "styles": [{}, { "bold": true }, { "underline": true }],
        "hyperlinks": [{ "uri": DOCS }],
        "panes": [wire]
    })
    .to_string()
}

fn per_patch(total: Duration) -> Duration {
    total / u32::try_from(PATCHES).expect("a few patches")
}

#[test]
#[ignore = "a benchmark: cargo test -p cornercase-core --release -- --ignored --nocapture"]
fn parses_and_applies_a_thousand_patches() {
    let mut grid = applied(&screen());
    let patches: Vec<_> = (0..PATCHES).map(scrolled).collect();
    let (mut parsing, mut applying) = (Duration::ZERO, Duration::ZERO);
    for text in &patches {
        let start = Instant::now();
        let patch: Frame = serde_json::from_str(text).expect("a surface line");
        let parsed = Instant::now();
        assert_eq!(grid.apply(patch), Applied::Applied);
        assert!(!grid.changed_rows().rows.is_empty());
        applying += parsed.elapsed();
        parsing += parsed - start;
    }
    assert_eq!(grid.revision, PATCHES);
    println!(
        "{PATCHES} patches on 120x{ROWS}: {:?} parsing and {:?} applying per patch",
        per_patch(parsing),
        per_patch(applying)
    );
}

#[test]
#[ignore = "a benchmark: cargo test -p cornercase-core --release -- --ignored --nocapture"]
fn parses_a_thousand_complete_frames() {
    let text = screen().to_string();
    let start = Instant::now();
    for _ in 0..PATCHES {
        let frame: Frame = serde_json::from_str(&text).expect("a surface line");
        assert_eq!(frame.panes.len(), 1);
    }
    println!("{PATCHES} complete frames of 120x{ROWS}: {:?} parsing each", per_patch(start.elapsed()));
}
