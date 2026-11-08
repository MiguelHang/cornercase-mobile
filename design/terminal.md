# Terminal

Follows cornercase-gpui's `design/session-model.md` for the `view` and `surface` JSON and for applying frames and patches.

- **Passive by default**: the phone opens a `view` with its own size but does not claim the tab, so the PTY keeps the Mac window's width and the Mac is never resized. The phone shows it with zoom and horizontal scrolling.
- **"Fit to this phone"** claims the tab by hand (`view-set --claim`) while that screen is open and releases it (`view-set --inactive`) on leaving. In cornercase, the latest claim sizes the tab and the window takes it back when it is used.
- Rejected: always claiming (the agent redraws at the phone's width and the Mac shows it cropped), never claiming (no way to read at the phone's width).
- Frames are shown only when their `projection` matches the latest known for the view.
- A row of keys the phone keyboard lacks (Esc, Tab, Ctrl, arrows) sits above the keyboard.
- **Frames and patches are applied in Rust** (`crates/core`, a port of gpui's `model/grid.rs`), tested on Linux; the WebView only gets resolved rows. Rejected: applying them in TypeScript (gpui's logic rewritten and tested a second time).
