# Upstream gaps

What the app cannot do because cornercase lacks it. No workarounds: each gap names the upstream issue or the proposal.

- **Conversation history**: cornercase gives only an agent's last message (`read --last-message`), so the chat view shows that, not a full history.
- **`/cli` endpoint**: the bridge stands in for it during development (see Bridge).
- **`requests --follow` and `input --follow` have no name in the contract**: they are subcommands of the CLI, not operations of the socket, so `contract --json` lists neither and the app cannot check them before using them; `REQUIRED` leaves them out, and 011 falls back to single commands when they do not start, telling a CLI that lacks them by clap's refusal (see Connection › The contract, and the ordered queue).
- **`requests` carries no bytes**: a line's `stdin` is text, so a command with bytes (`paste-file … -`, a pasted image) runs as its own command, a connection of its own, in its turn in the queue (see Connection › the ordered queue).
- **A pane's status does not say since when**: `status` gives each pane's status without the time it took it, so the board knows it only for the changes the app saw as `status` events; an agent already in its status when the app connects shows no time, and every time is dropped when the app may have missed events (a new `events` stream, a `dropped` event; see Board › Since when). Proposal: a `since` per pane, the RFC 3339 time of its last change, in `status --json` and its `--follow` lines.
