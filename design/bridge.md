# Bridge

A development-only binary in `bridge/` that runs on the Mac. Its purpose is to fix the format that cornercase-terminal will later serve as `/cli` in `src/web/`.

- **WebSocket ↔ `cornercase <args>`**, one WebSocket connection per command, so streams (`status --follow`, `surface --patches`, `events`) and one-shot commands share one shape and one lifetime.
- **Protocol**, one JSON object per message:
  - client → `{"argv": [...]}`, the first message, required;
  - client → `{"stdin": "<line>"}`; bridge → `{"stdout": "<line>"}` and `{"stderr": "<line>"}`, line by line;
  - bridge → `{"exit": n}`, then it closes.
  - client → `{"stdin_base64": "…"}` for bytes and `{"eof": true}` to close stdin: `paste-file -` sends image bytes, and `send -` and `paste-file -` read until end of input. Rejected: binary WebSocket frames (two message shapes in the protocol for a 33 % saving on rare images) and a bridge command that stores the image on the Mac (it leaves the argv ↔ cornercase contract).
  - `stderr` was added to the first draft so the app can show the CLI's errors.
- **Allowlist**: the `Kind::There` commands of cornercase-terminal's `src/cli/reach.rs` (the control subcommands, `stats`, and the `plugin` actions that ask the server). Out: `kill-server`, `update`, `restart`, `logs`, `cornercase` with no subcommand (it opens the TUI and needs a TTY) and `start-server` (it changes the Mac's server). A refused argv gets a `stderr` line and an `exit`, and nothing runs.
- **Disconnects kill the process** and its group.
- **Auth**: a fixed token in the URL (`?token=`), checked before the upgrade. No TLS and no pairing: development only.
- **Listening**: only on LAN or Tailscale interfaces, never `0.0.0.0` or a public address. On start it prints a QR code and the `ws://<ip>:<port>/?token=…` URL of each interface.
- **Proposal for upstream**: the last task documents this format as a proposal for `/cli` in cornercase-terminal.
