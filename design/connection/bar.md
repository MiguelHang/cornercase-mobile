# The connection bar

A topic of [Connection](../connection.md): how the app shows the active machine's connection.

- **Always in sight on `home`** (`src/lib/ui/ConnectionBar.svelte`), hidden on `no-machines`: fixed at the top across the safe area, on `--veil` with the glass blur and a `--border` line under it. Its height is measured and passed on, so `home` and the toasts start under it. A dot in the state's tone and one line of text, two while retrying, each cut with an ellipsis rather than wrapped, so the bar keeps its height. The text and tone come from `status` (`src/lib/connection.ts`), a pure function tested per state. Rejected: a toast per change, since the state lasts and a toast goes.
- **Per state** (Store › States):
  - `connecting`: `Connecting…`, in `--working`. The dot and the cursor of the mark on `home` pulse, in this state only (Look › Touch and motion).
  - `connected`: `<machine> · via <host:port>`, in `--done`. The route is cut to its address, without `ws://`, the path or the token, so the bar never shows the secret; a route that does not parse loses its query. The machine's name comes from `machines_list`.
  - `retrying`: `Retrying in N s`, with the error's message on the second line, in `--failed`. N counts down once a second from when the state arrived, rounded up and never below 1: `in_ms` is fixed in the event, and after a while in the background the local count could pass zero before `connecting` comes. The timer runs only while this state shows, so nothing ticks at rest.
  - `missing`: `cornercase on the Mac lacks <names>: update it`, with the names the contract lacks, in `--failed`.
  - `unauthorized`: `The bridge rejected this phone's token`, in `--failed`, and no fix offered. The fix is adding the route again with the new token, which needs 027 (the QR and a route added to a saved machine); pasting the URL again would make a second machine, and `home` has no form. A known limit until 027.
  - `incompatible`: `Incompatible: <message>`, in `--failed`.
  - `none` reads `Not connected`, though `home` does not show it: the route goes back to `no-machines`.
