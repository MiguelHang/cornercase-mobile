# The connection bar

A topic of [Connection](../connection.md): how the app shows the active machine's connection.

- **Always in sight on `home`** (`src/lib/ui/ConnectionBar.svelte`), hidden on `no-machines`: fixed at the top across the safe area, on `--veil` with the glass blur and a `--border` line under it. Its height is measured and passed on, so `home` and the toasts start under it. A dot in the state's tone and one line of text, two while retrying, each cut with an ellipsis rather than wrapped, so the bar keeps its height. The text and tone come from `status` (`src/lib/connection.ts`), a pure function tested per state. Rejected: a toast per change, since the state lasts and a toast goes.
- **Per state** (Store › States):
  - `connecting`: `Connecting…`, in `--working`. The dot and the cursor of the mark on `home` pulse, in this state only (Look › Touch and motion).
  - `connected`: `<machine> · via <host:port>`, in `--done`. The route is cut to its address, without `ws://`, the path or the token, so the bar never shows the secret; a route that does not parse loses its query. The machine's name comes from `machines_list`; while it has none (the list failed, or has not come back after `Add`), `via <host:port>` alone, with no separator hanging before it.
  - `retrying`: `Retrying in N s`, with the error's message on the second line, in `--failed`. N counts down once a second from when the state arrived, rounded up and never below 1: `in_ms` is fixed in the event, and after a while in the background the local count could pass zero before `connecting` comes. The timer runs only while this state shows, so nothing ticks at rest. The bar is a `status` live region, so the line with the number is hidden from readers and a still `Retrying` is read instead, with the error's line: VoiceOver would read the count again every second.
  - `missing`: `cornercase on the Mac lacks <names>: update it`, with the names the contract lacks, in `--failed`.
  - `unauthorized`: `The bridge rejected this phone's token`, in `--failed`, and no fix offered. The fix is adding the route again with the new token, which needs 027 (the QR and a route added to a saved machine); pasting the URL again would make a second machine, and `home` has no form. A known limit until 027.
  - `incompatible`: `Incompatible: <message>`, in `--failed`.
  - `offline` (028): `No network`, in `--failed`, with no pulse and no countdown: the store waits for the network rather than retrying.
  - `none` reads `Not connected`, though `home` does not show it: the route goes back to `no-machines`.
- **Checked on 2026-11-09** in the iPhone 17 simulator, iOS 27.2, against `cargo run -p cornercase-bridge` on the LAN and cornercase 0.13.19:
  - after `Add`, `Connecting…` for under half a second, then `MacBook · via 192.168.1.135:8765` and the pane count;
  - with the bridge stopped by `SIGSTOP`, a relaunch stayed in `connecting`, the dot and the cursor pulsing (their colours, read in screenshots, rose and fell), then went to `Retrying in 1 s` with `the bridge did not answer in time`;
  - with the bridge killed, `Retrying in N s` with `cannot reach the bridge: Connection refused (os error 61)` under it, counting down a second at a time, the wait growing between attempts (2, 8, 15 s), and no toast; started again on the same port, the app reconnected on its own.
