# Changes and files

A workspace's git changes from the phone, usually an agent's worktree: the list of changed files, a file's diff, and its viewed mark, which the Mac's changes panel shares (task 025). Code: `crates/core/src/cli/changes.rs` (the types and the argv), `crates/core/src/changes.rs` (following the list), `crates/core/src/follow.rs` (the reopening loop the terminal shares), `src-tauri/src/commands/changes.rs`, `src-tauri/src/commands/reach.rs`, `src/lib/changes.svelte.ts`, `src/lib/diff.ts` and `src/routes/changes/`. Upstream's `design/commands-for-scripts-and-agents/changes.md`, `src/cli/args/changes.rs` and `tests/fixtures/control/changes/v1/`, and gpui's `design/changes-panel.md` and `src/panels/changes/`, were read on 2026-11-09 (upstream at 425b3ae1, which matches 0.13.19 for these files). The files' own screen (026) will join this area.

## What is read

- **One type for `changes list --json` and every line of `changes list --follow`** (`Summary`, `ChangedWorkspace`, `ChangedFile`): upstream prints the same object both ways. `changes file --json` is a `Patch`: the file's fields flattened beside `workspace`, `mode` and `hunks`, whose lines carry `kind`, `old`, `new`, `text`, `raw` and `emphasis`.
- **Read leniently, as gpui's `wire.rs`**: every struct is `#[serde(default)]`, unknown fields are ignored, and each closed enum (`State`, `Status`, `Fold`, `Staged`, `LineKind`) has an `Other` (`#[serde(other)]`), so a value a newer cornercase adds reads as `Other` instead of rejecting the line. The UI shows `?` for a status it does not know and a line of an unknown kind as context.
- **`styles` and `syntax` are not read**: they are indexes into the Mac terminal's palette, which the phone has only inside a `surface`. Highlighting is left out (Left out).
- **The digest is text**, 16 hex digits, never a number: upstream sends it as a string because JSON numbers lose precision past 2^53 in JavaScript, and the app hands it back as it came.
- **Fixtures copied byte for byte** from upstream's `tests/fixtures/control/changes/v1/` (`summary.json`, `patch.json`, `every.json`) to `crates/core/tests/fixtures/control/changes/v1/`, read through `cli/changes/fixtures.rs`.

## The commands

- **`changes list --follow --workspace W --mode M`** for the list, **`changes file --workspace W --mode M --json -- PATH`** for a diff, **`changes viewed --workspace W --digest=D -- PATH`** to mark and **`changes viewed --workspace W --clear --mode M -- PATH`** to clear. `--` before the path and `--digest=` keep a path or a value that starts with `-` from being read as an option, as in Starting work.
- **`Mark { Seen { digest }, Clear { mode } }`**, so the type rules out what clap refuses: `--digest` conflicts with `--clear` and with `--mode` (the mode only chose which version a read showed), and clearing takes no digest. `changes_viewed` with `viewed: true` and no digest answers `no_digest` without sending anything.
- **The mode is the screen's**: upstream gives each stream its own mode, and `changes-mode` changes it only on the connection that holds the stream, which a CLI invocation never is, so another mode is another stream.

## Following the list

- **`--follow`, not `events`**: the app's `events` stream says nothing of git (no event when a file changes), so reading again on events would miss every edit. `changes list --follow` sends a whole new summary each time the files change, at most every 100 ms, and asks git at most once a second; the ticket allowed either, and only this one keeps the list live.
- **One stream per workspace and mode** while the list (or a diff over it) shows, in a slot of `Active` of its own (`follow`, `unfollow`), apart from the terminal's, so a diff and a terminal never stop each other. Following another workspace or mode replaces the task, `unfollow` of another workspace leaves it, and connecting again or removing the machine ends it with `Active`. Each summary replaces the list; none is a patch.
- **A first `loading` without files is ignored once a list was sent** (`only_starting`): every stream opens with one while git runs, and after a reconnection it would empty the list for a moment. The first open, and a new mode, show it as the spinner.
- **Opened again like the terminal** (Terminal › Opening it again), through `follow::reopen`, whose loop (`follow::revisit`) `Terminal::run` shares: wait until the `Store` is `Connected` with a session; a workspace missing from it ends the screen (`Closed`); otherwise open the stream on the `Store`'s transport, and drop it once the `Store` is no longer connected on that transport. After any end it waits a second (`SETTLE`) before looking at the session again, since a stream ends with `Detached` (workspace closed, project closing) or status 0 alike, and a restart closes nothing until a new session lists the workspace. Rejected: a second copy of the loop for the changes, which jscpd refuses and which could drift from the terminal's.
- **`state: failed` is a summary** (a folder that is not a repository, git's error): the screen shows its `error`, and the stream stays open, since cornercase sends a new summary when that changes; nothing retries in a loop. A line that does not parse, or a stream that fails, is a warning, once per run of failures, as an error `toast`.
- **`changes-changed { machine, workspace, mode, summary, closed }`**: a summary, or `summary: null` with `closed: true` when the workspace is gone, and the screen says `This workspace was closed on the Mac` and rests. The UI drops an event of another machine, workspace or mode. Failures go out as `toast`, the `Store`'s path, rather than a field of the event.

## Reading a diff

- **One-shot, on the transport**, as the chat's `last_message`: `changes file` writes nothing, so it does not wait behind sends in the queue.
- **Read again when its file's `digest` changes** in a summary that is `ready`, so an agent's edit shows without a tap; the digest kept is the row's at the time of the read. When the file leaves the list, the diff says `<path> no longer has changes` (with an `info` toast) and is not read again.
- **The newest read wins**: each read and each close takes a generation, and an answer of an older one, or for another file, is dropped (two quick taps, a read under a newer summary).
- **`binary` and `too large`** come without hunks; the screen says so in the diff's place. A `lockfile` is read and shown like any file, with its tag in the list.

## Viewed

- **Through the ordered queue** (`Queue::ask`), as every write: `Skipped` is `not_sent` and `Unanswered` `maybe_sent` (`commands/reach.rs`, which the chat and the start screen share), and the UI takes the reason as the chat does (`deliver`).
- **Optimistic**: the mark flips at once. A failed command takes it back to what the list said and shows the reason in an error toast. Once the command answered, the next summary decides (it carries the marks), so a mark another client changed meanwhile is not overwritten; a mark still on its way stays over the summaries that come before its answer.
- **The digest sent is the version shown**: the patch's while its diff is open, else the row's. It depends on the mode (Uncommitted's is a fingerprint of the file's metadata, Commits' and All's the patch's), while the mark is per file and shared by the modes, so it is the digest of the mode on screen.
- **A digest that is no longer current answers OK and marks nothing**: upstream stores marks without reading the file, so `changes-viewed` gives no `conflict`, and a digest read while the file was being written names no version at all. There is no error to take back; the next summary shows the file unmarked. Accepted: the alternative, reading the file again before marking, narrows the race without closing it, as upstream says.

## The screens

- **`changes`**, pushed by the agent's `Changes` (Chat › The screen) with the workspace and the name of its row: the name, `+N −M` for the workspace, the three modes (`Uncommitted` by default, `Commits`, `All`), one row per file and the states. Three modes because an agent that has already committed in its worktree leaves Uncommitted empty, and the work to review is in Commits, against the base. Empty, it says `No uncommitted changes`, `No commits ahead of <base>` or `Nothing changed since <base>`.
- **A row**: the status letter (`M`, `A`, `D`, `R`, `U`), the path or `old → new`, the `lockfile`, `binary` or `too large` tag, `+a` and `−r` when not zero, and the viewed box (`square`, `square-check`), a 44 px target of its own beside the row's button, which opens the diff. A viewed row is dimmed.
- **`change`**, pushed over it: the path in the header, cut at its start (`direction: rtl` with the text `ltr`), the viewed box, and the unified diff: each hunk's `@@ -a +b @@ context`, then each line with its old and new numbers, its sign and its text.
- **One box scrolls sideways, whole**: lines never wrap (`white-space: pre`), since a wrapped diff on a phone loses its columns; the numbers' gutter is `position: sticky` at the left, so they stay while the text scrolls. Added and removed lines take `--diff-added` and `--diff-removed`, and the words that changed (`emphasis`) the `-strong` tints: `--done` and `--failed` at 10 % and 32 %, tokens of our own, since gpui's theme has none for a diff.
- **`emphasis` counts characters**, Unicode scalar values as upstream does, so the text is split with `Array.from` and never by UTF-16 units, which an emoji would shift.
- **The `ChangesState` lives on** while `changes` or `change` is on top (`Router.reviewed`), so going back from a diff keeps the list and its stream; the agent's `Chat` lives on under both (`Router.opened`). Leaving the list unfollows. Its `changes_follow` and `changes_unfollow` are ordered as the terminal's `Opening` orders its calls (Terminal › Opening it): `changes_follow` is asynchronous in Tauri and `changes_unfollow` is not, so an unfollow sent right after a follow could run first and find nothing in the slot, and two follows sent at once (a quick switch of mode) could reach the slot in either order and leave the mode no longer shown. So the follows go one after the other (`#turn`), each skipped once the screen has left or another mode was asked for, and one that answers after the screen left is unfollowed again. The diff's `change` route carries no mode: the diff shows in the `ChangesState`'s, the list's.
- **A large diff is all in the DOM**: no virtual list. Upstream already folds a file that is too large, so what comes stays within its limit; noted here to look at should a long file scroll slowly.

## Left out

- `changes gap` (the unchanged lines between hunks), `stage`, `unstage`, `discard`, `base`, `merge-check`, `shared`, `--every` and `changes-mode`.
- Highlighting (`styles`, `syntax`), copying a line (`raw`), selecting, asking an agent, opening in an editor, a filter by text, and a link from a file to its whole content (026).
- A workspace without an agent: the board lists only agents, so there is no way in yet.

## Checked live

- **On 2026-11-09** in the iPhone 17e simulator (iOS 27.2, Xcode 27.2, macOS 26.6.2, Tauri 2.12.5), the app built with `tauri ios build --debug`, against the bridge on the LAN and cornercase 0.13.19, the taps made by XCUITest. A Claude Code 2.1.321 agent ran in a worktree of a small repository, with a modified file, a new one, a rename, a deletion and, later, an untracked one.
- **The list**: the agent's `Changes` opened it on `Uncommitted`, with `+6 −2` and its four rows (`R README.md → GUIDE.md`, `D gone.txt −1`, `A lib.rs +3`, `M main.rs +3 −1`), as `changes list` printed them on the Mac.
- **A diff**: `main.rs` showed its hunk with both columns of numbers, the removed line and three added ones, and the changed words of `println!("hello, café 🌍")` tinted exactly over `, café 🌍`, the emoji included. A line of `lib.rs` wider than the screen ran off its right edge unwrapped.
- **Viewed**: the box in the diff's header marked `main.rs`, and `changes list` on the Mac said `viewed` within two seconds; the box of a row marked `lib.rs` the same way, the row dimmed. The Mac's changes panel draws the marks of that same list; the window itself was not looked at.
- **Live**: a new file and a line added to `lib.rs` on the Mac showed in the list within five seconds, with `U answer.rs +1` and the totals, without a tap. A line added to `main.rs` cleared its mark on both sides, since its digest changed.
- **Modes**: after a commit on the Mac, `Commits` listed the rename and the deletion (`+0 −1`) and `All` the five files (`+10 −2`), as `changes list --mode commits` and `--mode all` printed them. Marks follow each mode's digest: `lib.rs`, marked in `Uncommitted`, showed unmarked in `All`.
- **Clearing**: the box of a marked row cleared it, and the Mac's list dropped `viewed`.
- **Closed**: closing the workspace on the Mac while its list showed turned the screen into `This workspace was closed on the Mac.`
- **Once**, the first diff read of the session answered `workspace 812 is not in a git repository` (the server's log says `refused … command=changes-file`), while the stream of the same workspace was `ready`. Opening it again a minute later worked, and neither the same command on the Mac, six at once, nor a new worktree read at once reproduced it; not reported upstream yet, since it was not reproduced.
