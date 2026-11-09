# Upstream gaps

What the app cannot do because cornercase lacks it. No workarounds: each gap names the upstream issue or the proposal.

- **Conversation history**: cornercase gives only an agent's last message (`read --last-message`), so the chat view shows that, not a full history.
- **`/cli` endpoint**: the bridge stands in for it during development (see Bridge).
- **`requests --follow` and `input --follow` have no name in the contract**: they are subcommands of the CLI, not operations of the socket, so `contract --json` lists neither and the app cannot check them before using them; `REQUIRED` leaves them out, and 011 falls back to single commands when they do not start (see Connection › The contract).
