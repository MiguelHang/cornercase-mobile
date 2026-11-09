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
- [Code layout](design/code-layout.md) (`.github/`, `clippy.toml`, `eslint.config.js`, `scripts/`, `.jscpd.json`): no comments, 1,000 lines per file, small functions and how clippy and ESLint count them, what `scripts/check-rules.mjs` checks, jscpd, `pub(crate)`, what CI runs and in which order, Dependabot.
- [Bridge](design/bridge.md) (`bridge/`, `crates/core/src/protocol.rs`): the development bridge, its protocol, the allowlist, running a command and ending its process group, the WebSocket server (auth, ping, limits, stopping, logs, `websocat`), listening on the LAN and Tailscale, the token, the URL and its QR code, the proposal for `/cli`.
- [Connection](design/connection.md) (`crates/core/`, `src-tauri/src/secrets.rs`, `src-tauri/src/identity.rs`): the command line as the only contract, the transport (its trait, errors, cancelling, the bridge transport, `FakeTransport`), machines and routes, the identity and its seed in the Keychain, `contract` and `boot`, the ordered queue (`requests --follow`, `input --follow`, coalescing, falling back, `reconnect`).
- [Terminal](design/terminal.md) (`src/`, `crates/core/`): the live terminal, passive by default, claiming the tab by hand.
- [Look](design/look.md) (`src/lib/theme/`, `src/lib/ui/`, `src/routes/`, `src-tauri/gen/apple/` for the launch): following cornercase-gpui's look on a phone, the tokens, glass on an opaque ground, touch, motion, system fonts, Lucide icons, the dark launch.
- [Upstream gaps](design/upstream-gaps.md): what the app cannot do because cornercase lacks it.
