# Board

The active machine's screen, under the connection bar: every agent of its projects with its status and how long it has been in it, the ones that need the user first, live. Follows cornercase-gpui's agents section (`design/session-model.md`: loudest status, `status_rank`, `agent_title`; `src/model.rs`, `src/sidebar/agents.rs`), read on `main` on 2026-11-09.

## Rows

- **One row per pane with an agent** (`crates/core/src/board.rs`, `AgentRow`), from every project of the session; a pane without one gives none. The rows are built in the core, beside the session and the record of since when (below), and only they reach the WebView, over `agents` and `agents-changed` (`design/architecture.md` › Commands and events). Rejected: grouping and sorting the session in TypeScript after a `session-changed`, which sent the whole snapshot over IPC about every 500 ms while an agent worked and left the order untested against upstream's fixtures.
- **Flat, one agent per pane.** gpui's tree of agents and the agents they started (`agent_rows`) was left out: a flow with subagents is one row per pane, without virtualisation, until a real list is too long.
- **Names as the Mac shows them**: a project's, workspace's and tab's `custom_name`, else its `name`. The row carries `workspace_count` rather than a place already written, so the UI writes the line as gpui's `place` does: the project alone when it has one workspace, else `project › workspace`. `tab_name` goes out too, but no screen shows it yet; it belongs to the chat's header (016).
- **The title is gpui's `agent_title`**: the pane's `title` without the characters before its first letter or digit (a spinner frame, `✳`), trimmed; an empty or missing one is `null`, and the UI shows the agent's name (`claude`) instead.

## Order

- **By status, then by time in it.** The rank is gpui's `status_rank`: `waiting` > `failed` > `done` > `pending` > `working` > `idle`, and a status the app does not know ranks as idle, so one upstream adds sorts last rather than failing.
  - **`done` above `pending`, as gpui, not as the TUI** (`activity::Status`, which puts `pending` first, since it is the user's own mark). gpui is the reference client, and on a phone a finished turn not seen yet is the news, while `pending` is a reminder the user set by hand that may stay for days.
  - **`shell` ranks with `working`**, unlike gpui's `status_rank`, which leaves it with idle: the agent wakes up when its background command ends, which is why the TUI and Claude Code's own agents view count it as working.
- **Within a rank, the longest in its status first**, with an unknown `since` before any known one: such an agent was already in that status when the app connected, so it has been there longer than any time the app saw. The times are compared as text: upstream writes them in UTC with milliseconds and a fixed width, so the text order is the time order, and the core needs no date library. Ties keep the session's order (project, workspace, tab, pane), since the sort is stable.

## Sections

- **Waiting** (`waiting`, `failed`), **Done** (`done`, `pending`), **Working** (`working`, `shell`) and **Idle** (`idle`, no status, an unknown one), in that order; an empty one is not drawn. They cut the rank in four, so the rows' order inside each is the core's (`src/lib/board.ts`). The names are in English, like the rest of the UI; the task's were Spanish.
- **Idle has its own section, last.** Rejected: idle inside Done, which dilutes "what finished and you have not seen", and hiding it, since an idle agent is still someone to write to from the phone.

## Since when

- **From the `status` events, since the session has no time.** cornercase's `status` gives each pane's status, not when it took it (Upstream gaps); each `status` event carries its `time`. The `Store` keeps a record per pane (`board::Since`, Connection › Store): an event sets its `(to, time)`; a snapshot whose status for a pane differs from the record sets `(status, null)`; a later event of that same status gives it its time. So the event may come before or after the snapshot that shows the change. A pane gone from the snapshot leaves the record, and a restart empties it. In the `Store` rather than in the forwarder, so the `agents` getter sees it too.
- **The limit**: the app knows the time only of the changes it saw. An agent already in its status when the app connects, or one that changed while the app was away or in a `dropped` gap of the events, shows no time, and sorts first in its rank, which is right: it has been there longer.
- **How it reads**: `now` under a minute, then `N min`, `N h` and `N d`, rounded down. A time ahead of the phone's clock, from a Mac whose clock is ahead, reads `now`. One timer every 30 s while the board is mounted gives every row the same `now`, and stops when it unmounts. Rejected: a timer per row.

## Live, without flicker

- **The rows go out only when they change** (Connection › Store › In the app), so a snapshot every 500 ms with the same rows redraws nothing.
- **One flat list of headers and rows**, keyed `section:<name>` and `pane:<id>`, with `animate:flip`. Svelte's `flip` moves an element only within its `{#each}`, so with one list per section a row going from Working to Waiting would vanish from one and appear in the other; in one list it slides to its new place, and the headers slide with it. Rejected: one list per section with `crossfade`, two transitions to keep in step for the same effect. The slide lasts `ENTER_MS`, 0 with Reduce Motion.
- **Before the first snapshot** (`agents` is `null`: connecting, or after a restart) the screen shows the mark, pulsing while it connects; with no agents, `No agents running.`

## A row

- **A button of at least 44 px** with an empty `ontouchstart` (Look › Touch and motion): the `StatusDot` (Look › Components), the title, the place under it in `--muted`, and the time on the right in tabular figures, so the digits do not jump as it counts; no time when `since` is `null`.
- **A tap opens the agent's screen** (`router.go({ name: 'agent', pane })`, Look › Navigation): here a skeleton of 016's chat with its dot, title and place, read by pane from the latest rows, and `Back`. While the agent works its dot keeps changing there too. A pane no longer among the rows says `This agent is no longer running.`; 016 decides the final text.

## Left out

- The agents of the other servers a Mac links (`status`'s `machines`, gpui's `machine_agents.rs`): their ids belong to another server, and the app's `Snapshot` does not read them.
- Notifications, folding the sections, filters, search, and the chat itself (016).
