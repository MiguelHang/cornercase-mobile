# Starting work

New work on the Mac from the phone: an agent with a prompt, in its own worktree when a branch is given, or a workspace without one (task 023). Code: `crates/core/src/cli/agents.rs` (`account list --check --json`), `crates/core/src/cli/start.rs` (the argv and the ids that answer), `src-tauri/src/commands/start.rs`, `src/lib/startForm.svelte.ts` and `src/routes/start/`. Upstream's `StartArgs` and `NewWorkspaceArgs` (`src/cli/args.rs`) and gpui's `src/sidebar/forms/agents.rs` were read on `main` on 2026-11-09.

## Where the choices come from

- **The projects are the session's**, as the `Store` last read them (`start_projects`, `ProjectChoice { id, name }`): the name the Mac shows (`custom_name`, else `name`), in the session's order. No call of its own, so they show at once; with one project, it is chosen.
- **The agents are cornercase's**: `account list --check --json` through the queue (`installed_agents`), its `agents` field, only those with `installed: true`, in cornercase's order. The app keeps no list of agents, so one cornercase adds shows up without a release of the app. An absent `agents` (a server with `account-list` but older than cornercase-terminal#543) reads as none installed. The rest of the answer (providers, accounts, paths) is not read, and the fixture keeps only `agents`.
- **They load apart**: `account list --check` runs each account's status command when accounts were added (up to 20 s each, upstream's `design/accounts-and-profiles.md`), so the projects and the other fields can be filled while the agents load. The agents' picker says it is reading, shows cornercase's reason with `Try again` when it fails, and says when none is installed, which holds `Start`.
- **Which agent is chosen**: the first installed one (claude comes first in cornercase's order), then the last one started from the app while it runs, if it is still installed. Rejected: the `agent` of `config --json`, another call with a large answer, useless when it is `auto`; and no default, one more tap almost every time.
- **The model is free text**, optional: cornercase says nothing of the models each agent takes (Upstream gaps), and a list in the app would go stale.
- **After a server restart** (a new `boot` while the screen is open) the choices are read again, since ids belong to one run of the server and `start` takes no `--boot`. A chosen project that is gone is emptied; every other field stays.

## Starting an agent

- **`start <agent> --project <id> [--worktree=<branch>] [--model=<model>] --prompt=<prompt> --placed --json`**, through the ordered queue (`Queue::ask`). Values are trimmed, and an empty model or branch is not sent. With a branch, the agent works in a new worktree on it; without one, it starts in a new tab in the project's folder, which the screen says under the branch.
- **`--x=value` and `--` before a positional**, so a value that starts with `-` (a prompt such as `--help me`, a branch, a workspace's name) is not read by clap as an option.
- **Always an explicit agent**: without one, `start` uses `auto`, the agent the caller runs in, which means nothing from a phone.
- **`--placed`** answers once the tab is open (`project`, `workspace`, `tab`, `pane`, an optional `warning`), before the project's setup runs and the agent is typed. Without it, `start` waits for the agent to take the prompt, and a folder's trust question would hold the queue, and every send behind it, until someone answers it.
- **No `--focus`**: gpui sends it because it is the window; whoever uses the phone is seldom at the Mac, and moving the window under someone who is would surprise them.
- **The prompt travels as one argument** of the argv, inside the JSON line of `requests --follow`, with no limit of the app's own. Should the bridge or a later `/cli` limit the size of an argv, `--prompt-file -` with the prompt on standard input is the way out, which the queue already carries.
- **The ids answer is read leniently** (`Started`, `#[serde(default)]`), with fixtures of 0.13.19 in the shape of upstream's `control::Ids`, with made-up ids.

## Creating a workspace

- **`new-workspace --project <id> [--worktree] --json -- <name>`**: the project, the name and the `Own worktree` switch, whose name is then the branch. It has no agent, so there is no row and no screen to open: the app says `Workspace <name> created` in an `info` toast and goes back to the board. Rejected: staying on the form to create another or to start an agent there, which needs `start --workspace` (left out).

## The screen

- **A screen of its own, `start`, pushed over the board** by the board's `Start` button, which rests while the connection is not `connected` (Look › Navigation). Rejected: the ticket's sheet from the bottom, since the prompt always opens the keyboard and WKWebView covers what is fixed at the bottom (Look › Forms).
- **Two actions, `Agent` and `Workspace`**, two buttons with `aria-pressed` that switch between two forms (`AgentForm`, `WorkspaceForm`), each with its own button. One `StartForm` holds both, so the project chosen in one is the other's too.
- **Checked before sending**, each error under its field: a project, an agent, a prompt that is not only spaces, a workspace's name. A server is not asked what the phone can tell.
- **While it sends**, the button shows the spinner and `Starting…` (or `Creating…`), the fields rest, and nothing can send again. Outside `connected`, the buttons rest and a line says that what was typed stays until the app connects.
- **A refusal keeps everything typed** and shows cornercase's reason over the button, its stderr without `Error:` (`CommandError::reason`'s rule, local to `start.rs`), not only in a toast: `--worktree` in a folder that is not a repository's root, for one, which the session cannot tell beforehand.
- **`Unanswered` is never retried**: the line may have run. It says that the agent may have started, or the workspace may exist, and to look before trying again.

## After it starts

- **The agent's screen takes the form's place** (`router.replace`, `{ name: 'agent', machine, pane, starting: true }`), so `Back` goes to the board and not to a form already sent. A `warning` in the answer shows as an `info` toast.
- **`Starting <agent>…`** with the spinner, until the pane's row comes in `agents-changed`: at the answer, the pane holds no agent yet, and the rows list only panes with one. Without it, the screen would say `This agent is no longer running.` at the first list (Chat › Gone). While it starts, a list without the pane does not count it as gone; once its row was seen, the usual rules hold, and another machine still ends it. Rejected: staying on the form until the row comes, with a limit, since a project's setup can take minutes and the form would be held all that time.
- **No limit on the wait**: if the agent never starts (its setup fails, its command is not found), the screen keeps saying it starts, and `Back` goes to the board, where the tab's shell shows no row.
- **The agent's name is kept by `App`**, the last one started, since the route says only `starting`.

## The trust question

- With `accept_trust_prompts` off, cornercase's default, an agent in a new worktree (a new folder) asks first whether to trust it, and the prompt waits for the answer. It is answered from the phone with Enter in the chat's keys (017) or in the terminal (021); until those exist, the agent's screen has `Terminal` (020, passive), which shows the question but cannot answer it. `--placed` keeps it from holding the queue.

## Checked live

- Not checked live yet (task 023's T12).

## Left out

- Starting from an issue (024), `--machine`, `--mode`, `--effort`, a tab's `--name`, profiles and accounts (`--profile`), `queue add` and flows.
- Opening the terminal of a workspace without an agent: the `terminal` route needs an agent's row.
