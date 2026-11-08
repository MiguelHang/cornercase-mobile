# cornercase-mobile

> iOS and Android app (Tauri 2) to follow, answer and start cornercase agents from a phone. It talks to
> the cornercase server only through its command line, the public contract, like cornercase-gpui.

## Context
- Idea and requirements: `docs/IDEA.md`
- Roadmap and tasks: `docs/ROADMAP.md`, `docs/tasks/`
- Design decisions: `DESIGN.md` (index) and `design/`, one file per area. Read the area's file before
  changing it; record what was decided and why (and what was rejected) in the same PR.
- References to follow, not to copy blindly:
  - `usecornercase/cornercase-gpui`: `design/widgets.md` + `src/theme.rs` (look), `design/connection.md`
    (connection), `design/session-model.md` (terminal JSON, frames and patches).
  - `usecornercase/cornercase-terminal` on upstream `main` (the local `~/me/cornercase-terminal` may lag):
    `src/cli/reach.rs`, `design/commands-for-scripts-and-agents.md` and its topics, `design/remote-windows.md`,
    `design/browser-access.md`.
  - `pingdotgg/t3code` (`docs/user/remote-access.md`), `stablyai/orca` (`mobile/`).

## Stack
- Tauri 2 (iOS first, then Android), Rust stable, Cargo workspace.
- Svelte 5 + Vite + TypeScript in the WebView, pnpm.
- tokio, tokio-tungstenite, ed25519-dalek, tauri-specta (TypeScript types from Rust).
- No network code in the WebView: everything goes through Tauri commands and events.

## Commands
```sh
pnpm install                                      # frontend packages and the Tauri CLI (pnpm from packageManager)
pnpm build                                        # dist/: run it before cargo touches src-tauri, as CI does
pnpm tauri ios dev "<iPhone>"                     # the app in the iOS simulator; names from `xcrun simctl list devices`
cargo run -p cornercase-bridge                    # the development bridge, on the Mac
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
pnpm check                                        # svelte-check
pnpm lint                                         # eslint
pnpm test                                         # vitest
```

- **The toolchain is pinned** in `rust-toolchain.toml`, with the two iOS targets; rustup installs it on the first
  cargo command. Node 22.
- **CI runs the list above, in that order** (`design/code-layout.md`). Before calling a task done, run it and fix
  what fails.
- **iOS needs the full Xcode**, not the Command Line Tools: `sudo xcode-select -s /Applications/Xcode.app`,
  `sudo xcodebuild -license accept` (until then `git` and `clang` refuse to run), open Xcode once and download an iOS
  simulator runtime. Install CocoaPods or xcodegen (`brew install cocoapods xcodegen`) only if `pnpm tauri ios init`
  asks for them.
- Android has no project yet (task 030).

## Structure
- `crates/core/`: pure, testable logic: transport trait, bridge protocol, CLI types over `--json`,
  ordered queue, `Store`, machines and routes. No Tauri, no platform code.
- `src-tauri/`: the app shell: Tauri commands and events, key storage (Keychain / Keystore), lifecycle
  and network-change hooks, mobile projects.
- `bridge/`: the development bridge binary (WebSocket ↔ `cornercase <args>`), runs on the Mac.
- `src/`: the Svelte UI.
- `design/`, `DESIGN.md`: decisions by area. `docs/`: idea, roadmap and tasks.

## Code rules
- No comments, in Rust or TypeScript. Names and small functions say what the code does; the why goes
  to `design/`.
- At most 1,000 lines per file. Small functions with one purpose.
- Rust: `pub(crate)` by default; `pub` only for what another crate of the workspace uses.
- The CLI is the only contract with cornercase: never read its files, sockets or internal state.
- No workarounds for what cornercase lacks: note the gap in `design/upstream-gaps.md`.
- No new dependency without a line in the area's `design/` file saying why.
- No dead code.

## Tests
- `crates/core` and `bridge/` carry the logic and its unit tests (protocol, allowlist, queue, Store,
  applying frames and patches, route selection), with JSON fixtures from cornercase's `--json` output.
- UI logic that is not markup (stores, formatting, key mapping) has vitest tests.
- Every task closes with tests green and fmt, clippy, svelte-check and lint clean.

## Workflow
- One task in `docs/tasks/` = one branch = one PR.
- Commits: Conventional Commits.
- This repo uses the harness (`/hx-start`, `/hx-build`, `/hx-review`, `/hx-ship`).
