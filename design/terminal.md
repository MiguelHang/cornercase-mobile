# Terminal

Follows cornercase-gpui's `design/session-model.md` for the `view` and `surface` JSON and for applying frames and patches.

- **Passive by default**: the phone opens a `view` with its own size but does not claim the tab, so the PTY keeps the Mac window's width and the Mac is never resized. The phone shows it with zoom and horizontal scrolling.
- **"Fit to this phone"** claims the tab by hand (`view-set --claim`) while that screen is open and releases it (`view-set --inactive`) on leaving. In cornercase, the latest claim sizes the tab and the window takes it back when it is used.
- Rejected: always claiming (the agent redraws at the phone's width and the Mac shows it cropped), never claiming (no way to read at the phone's width).
- A row of keys the phone keyboard lacks (Esc, Tab, Ctrl, arrows) sits above the keyboard.

## The grid

Code: `crates/core/src/cli/surface.rs` (the `surface` JSON), `crates/core/src/cli/view.rs` (a `view` line), `crates/core/src/terminal/` (the grid).

- **Frames and patches are applied in Rust** (`crates/core`, a port of gpui's `model/grid.rs`), tested on Linux; the WebView only gets resolved rows. Rejected: applying them in TypeScript (gpui's logic rewritten and tested a second time, and every patch crossing the bridge whole), and `alacritty_terminal`, a VT emulator that eats a PTY's bytes: cornercase sends cells Ghostty has already parsed, and no command of the contract gives a PTY's bytes, so it would be a heavy dependency with nothing to feed it.
- **The rules are gpui's** (`design/session-model.md`): styles and links resolved on arrival, a patch applied only on the revision, surface and boot it was made from (else `NeedRedraw`, also for a pane the grid does not hold), the order within a pane (cursor always, `active`, `modes`, `scroll` and `colours` when present, `copy` from the lines as they were before it, then changed rows; a key that is not a number skipped). A patch leaves `rect` and `size` alone: cornercase sends a complete frame whenever they change.
- **Where it differs from gpui**:
  - Rows stop at the pane's `size.rows`: a frame's extra lines and a patch's rows past it are dropped. gpui grows the grid to any key, so a key like `"4000000000"` would reserve that many lines on a phone.
  - A run keeps its cells joined (`text`) and how many there were (`width`), not one string per cell: the WebView draws runs, and nothing hits a single cell yet. Tapping a link at an exact column may need the cells back.
  - Every type is `#[serde(default)]`, including a pane, a `copy` and the frame itself, so a field cornercase drops never rejects a frame.
  - No popup: plugin popups (`surface --popup`) come with a later task.
- **`projection`**: the view (`cli/view.rs`) tells the grid the highest projection it has seen (`saw_projection`), and a grid whose frame carries a lower one is stale. A stale frame or patch is still applied, so the chain of `base` and `revision` stays whole, but it shows nothing: `changed_rows` answers no rows. Once a frame or patch makes it current again, it answers every row of every pane, as after a complete frame. Rejected: throwing a stale frame away, which turns every patch made on it into a `NeedRedraw` and a needless `redraw` until the next complete frame.
- **What goes to the WebView**: `changed_rows` hands over, and forgets, the rows each pane changed since the last call, with `replaced` after a complete frame. A pane's cursor, modes, scroll, colours and whether it is active are not rows: whoever emits reads them from the grid each time, as they are small. After a `NeedRedraw` nothing is emitted until the complete frame: the panes patched before the unknown one hold half a patch.
- **The fixtures** (`crates/core/tests/fixtures/control/surface/v1/frame.json`, `patch.json`) are upstream's, from cornercase-terminal `origin/main` at `7f08441f` (0.13.19); the cases they do not cover are built in the tests.
- **Speed**: a thousand scroll patches (each copying 38 or 39 rows and adding 1 or 2) on a 120×40 pane took about 11 µs a patch on 2026-11-09 (Apple M4, release build), far under the millisecond that would matter, so `copy` still clones the whole pane first, as gpui does. The benchmark is `terminal::grid::tests::bench`, ignored; it runs with `cargo test -p cornercase-core --release -- --ignored --nocapture`.
