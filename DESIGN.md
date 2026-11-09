# Design decisions

Why cornercase-mobile works the way it does, area by area. The rules every change follows are in
[CLAUDE.md](CLAUDE.md). This file is their index: the notes are in [`design/`](design/), one file per area.

## How to use it

- Each line below gives an area's file, the code it covers and what it decides.
- A new area gets its file and a line here.
- A big area is also a folder, `design/<area>/`, of topics, listed at the top of the area's file.

## What goes in an area file

- What was decided and why, the alternatives that were rejected and why, limits that are not obvious,
  what waits upstream, and what was or was not checked live, with the versions.
- Not how the change was built, raw measurements (keep the conclusion and its date), or names and
  values the code already states.
- A file over about 40 KB is split by topic into `design/<area>/<topic>.md`: the area file keeps what applies
  to the whole area and lists the topics.

## Areas

- [Architecture](design/architecture.md) (`crates/core/`, `src-tauri/`, `bridge/`, `src/`): the workspace, what lives in Rust and what in the WebView, the bundle id, Svelte without SvelteKit, the native projects in git, native code in Tauri plugins of our own, the Tauri commands and events and their generated TypeScript bindings, the versions and each dependency.
- [Code layout](design/code-layout.md) (`.github/`, `clippy.toml`, `eslint.config.js`, `scripts/`, `.jscpd.json`): no comments, 1,000 lines per file, small functions and how clippy and ESLint count them, what `scripts/check-rules.mjs` checks, jscpd, `pub(crate)`, what CI runs and in which order, Dependabot.
- [Bridge](design/bridge.md) (`bridge/`, `crates/core/src/protocol.rs`): the development bridge, its protocol, the allowlist, running a command and ending its process group, the WebSocket server (auth, ping, limits, stopping, logs, `websocat`), listening on the LAN and Tailscale, the token, the URL and its QR code, the proposal for `/cli`.
- [Connection](design/connection.md) (`crates/core/`, `src-tauri/src/secrets.rs`, `src-tauri/src/identity.rs`, `src-tauri/src/state.rs`, `src-tauri/src/events.rs`, `src-tauri/src/lifecycle.rs`, `src-tauri/src/network.rs`, `src-tauri/plugins/network/`, `src/lib/connection.ts`, `src/lib/machineForm.ts`, `src/lib/addMachine.svelte.ts`, `src/lib/join.ts`, `src/lib/scanner.svelte.ts`, `src/lib/probes.svelte.ts`, `src/lib/ui/ConnectionBar.svelte`, `src-tauri/src/commands/routes.rs`, `src-tauri/src/commands/probe.rs`): the command line as the only contract, the transport (its trait, errors, cancelling, the bridge transport, `FakeTransport`), machines and routes, the identity and its seed in the Keychain, `contract` and `boot`, the ordered queue (`requests --follow`, `input --follow`, coalescing, falling back, `reconnect`), the `Store` (`crates/core/src/store.rs`, `store/`: the `events`, `status --follow` and `status --app --follow` streams, its states, the backoff and `resume_now`, restarts, the changes it sends; in the app, one `Store` for the active machine, its changes as events and `resume_now` on the return to the foreground), network changes (the iOS plugin over `NWPathMonitor`, the debounce, `Offline`), adding a machine from its URL, the connection bar, the Machines screen's side (scanning the QR code, the same machine, names, routes, the active machine remembered, switching, trying the others).
- [Terminal](design/terminal.md) (`src/`, `crates/core/src/terminal/`, `crates/core/src/cli/surface.rs`, `crates/core/src/cli/view.rs`): the live terminal, passive by default, claiming the tab by hand, the grid frames and patches build up in Rust, `projection`, the rows sent to the WebView.
- [Look](design/look.md) (`src/lib/theme/`, `src/lib/ui/`, `src/routes/`, `src/lib/router.svelte.ts`, `src/lib/toasts.ts`, `src/lib/toasts.svelte.ts`, `src-tauri/gen/apple/` for the launch): following cornercase-gpui's look on a phone, the tokens, glass on an opaque ground, touch, buttons, the spinner, motion and the pulse, the router and its base, the toasts, fields and the first machine's form, the Machines screens and the danger tone, the scanner's layer, system fonts, Lucide icons, the dark launch.
- [Board](design/board.md) (`crates/core/src/board.rs`, `src/routes/board/`, `src/routes/agent/`, `src/lib/board.ts`): the agents board, one row per agent, its order and why `done` comes before `pending`, the sections, since when from the events and its limit, one flat list for the reordering, what a row shows, the agent's skeleton screen, what was left out.
- [Upstream gaps](design/upstream-gaps.md): what the app cannot do because cornercase lacks it.
