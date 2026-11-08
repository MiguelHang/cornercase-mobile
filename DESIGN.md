# Design decisions

Why cornercase-mobile works the way it does, area by area. The rules every change follows are in
[CLAUDE.md](CLAUDE.md). This file is their index: the notes are in [`design/`](design/), one file per area.

## How to use it

- Each line below gives an area's file, the code it covers and what it decides.
- A new area gets its file and a line here.

## What goes in an area file

- What was decided and why, the alternatives that were rejected and why, limits that are not obvious,
  what waits upstream, and what was or was not checked live, with the versions.
- Not how the change was built, raw measurements (keep the conclusion and its date), or names and
  values the code already states.

## Areas

- [Architecture](design/architecture.md) (`crates/core/`, `src-tauri/`, `bridge/`, `src/`): the workspace, what lives in Rust and what in the WebView, the bundle id, Svelte without SvelteKit, the native projects in git, the versions and each dependency.
- [Code layout](design/code-layout.md) (`.github/`): no comments, 1,000 lines per file, small functions, `pub(crate)`, what CI runs and in which order, Dependabot.
- [Bridge](design/bridge.md) (`bridge/`): the development bridge, its protocol, the allowlist, listening, the proposal for `/cli`.
- [Connection](design/connection.md) (`crates/core/`): the command line as the only contract, the transport, machines and routes, identity, `contract` and `boot`.
- [Terminal](design/terminal.md) (`src/`, `crates/core/`): the live terminal, passive by default, claiming the tab by hand.
- [Look](design/look.md) (`src/`): following cornercase-gpui's look on a phone.
- [Upstream gaps](design/upstream-gaps.md): what the app cannot do because cornercase lacks it.
