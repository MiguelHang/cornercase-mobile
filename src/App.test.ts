import { render } from 'svelte/server'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { describe, expect, it, vi } from 'vitest'
import App from './App.svelte'
import appSource from './App.svelte?raw'
import noMachinesSource from './routes/NoMachines.svelte?raw'

vi.mock('@tauri-apps/api/core', async (actual) => ({ ...(await actual<object>()), invoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', async (actual) => ({ ...(await actual<object>()), listen: vi.fn() }))

describe('App', () => {
  const body = render(App).body.replaceAll('<!---->', '')

  it('opens on the screen with no machines, under the brand', () => {
    expect(body).toMatch(
      /<h1 class="name svelte-[\w-]+"><span class="wordmark svelte-[\w-]+" style="font-size: 22px;">cornercase<\/span><\/h1>/,
    )
    expect(body).toContain('--unit: 5px;')
    expect(body).toMatch(/<h2 class="title svelte-[\w-]+">No machines yet<\/h2>/)
  })

  it('says what a machine is and what it needs', () => {
    expect(body).toContain('A machine is a computer running cornercase whose agents you follow from here.')
    expect(body).toContain('It needs the cornercase bridge running on your Mac, and the code it prints.')
  })

  it('offers adding a machine, ready to be pressed', () => {
    expect(body).toMatch(/<button class="button primary svelte-[\w-]+" type="button">(<!--[^>]*-->)* Add machine<\/button>/)
  })

  it('neither calls nor listens to Tauri while rendering on the server', () => {
    expect(render(App).body).toContain('No machines yet')
    expect(invoke).not.toHaveBeenCalled()
    expect(listen).not.toHaveBeenCalled()
  })

  it('shows the empty toast stack but no connection bar while there is no machine', () => {
    expect(body).toMatch(/<div class="toasts svelte-[\w-]+" aria-live="polite" style="--below: 0px;"><!--\[--><!--\]--><\/div>/)
    expect(body).not.toContain('role="status"')
    expect(body).not.toContain('class="board')
  })

  it('brings the card in with the shared motion', () => {
    expect(noMachinesSource).toMatch(/class="card"[^>]*\sin:riseIn[\s>]/)
    expect(noMachinesSource).toMatch(/class="card"[^>]*\sout:fadeOut[\s>]/)
  })
})

describe('App wiring', () => {
  const source = appSource.replace(/\s+/g, ' ')

  it('takes the base route from the screen on every change, without leaving an open agent once the machine is gone', () => {
    expect(source).toContain('const router = new Router(routeFor(idle, 0))')
    expect(source).toContain(
      'const show = (next: Screen) => { current = next router.setBase(routeFor(next, machines.length)) }',
    )
    expect(source.match(/ current = (?!\$state)/g)).toHaveLength(1)
  })

  it('connects to the last machine asked for, and a refused connect becomes an error toast', () => {
    expect(source).toContain(
      "const join = latestWins(async (machine: number) => { show(connecting(machine)) const connected = await commands.connect(machine) if (connected.status === 'error') { toasts.failed(connected.error) } })",
    )
  })

  it('goes home with a new machine, reloads the machines for its name and connects to it', () => {
    expect(source).toContain(
      'const added = (machine: number) => { show(connecting(machine)) Promise.all([commands.machinesList().then(remember), join(machine)]).catch(toasts.failed) }',
    )
    expect(source).toContain("machines.find((machine) => machine.id === current.machine)?.name ?? ''")
  })

  it('reads the machines before the first screen, so saved machines never show the empty screen', () => {
    expect(source).toContain('const known = remember(listed) show(screen(state, agents))')
  })

  it('turns a failed machine list into an error toast and connects to nothing then', () => {
    expect(source).toContain("if (listed.status === 'error') { toasts.failed(listed.error) return { machines: [], active: null } }")
    expect(source).toContain('const known = remember(listed) show(screen(state, agents)) const first = firstToConnect(known, state)')
  })

  it('opens Machines from the bar, with the list, the active machine and its connection', () => {
    expect(source).toContain(
      "{:else if route.name === 'machines'} <Machines {machines} active={current.machine} connection={current.connection} {backTo} onback={goBack} onpick={(machine) => void join(machine)} onadded={added} onrouted={reload} onedit={(machine) => router.go({ name: 'machine', id: machine })} onrescan={(machine) => router.go({ name: 'machine', id: machine, adding: true })} />",
    )
  })

  it('edits the machine asked for, against the names of the others, and reloads the machines after each change', () => {
    expect(source).toContain(
      "const editing = $derived(route.name === 'machine' ? machines.find((machine) => machine.id === route.id) : undefined)",
    )
    expect(source).toContain(
      'const taken = $derived(machines.filter((machine) => machine.id !== editing?.id).map((machine) => machine.name))',
    )
    expect(source).toContain('const reload = () => commands.machinesList().then(remember).catch(toasts.failed)')
    expect(source).toContain(
      '{:else if editing !== undefined} <Machine machine={editing} {taken} connection={editing.id === current.machine ? current.connection : null} {backTo} onback={goBack} onchanged={reload} onremoved={removed} adding={rescanning} />',
    )
  })

  it('opens a machine with Add route open when Machines asks to scan it again', () => {
    expect(source).toContain(
      "const rescanning = $derived(route.name === 'machine' && route.adding === true)",
    )
  })

  it('leaves a deleted machine for Machines, connecting no other, and for the empty screen after the last', () => {
    expect(source).toContain(
      'const removed = async () => { router.back() await reload() router.setBase(routeFor(current, machines.length)) }',
    )
    expect(source.match(/join\(/g)).toHaveLength(3)
  })

  it('listens before loading, and sends toasts and failures to the stack', () => {
    expect(source).toContain('events.toast.listen((event) => toasts.push(event.payload)),')
    expect(source).toContain('events.agentsChanged.listen((event) => show(apply(current, event.payload))),')
    expect(source).toContain('Promise.all(listening).then(load).catch(toasts.failed)')
  })

  it('loads the agents with the state, and never the session', () => {
    expect(source).toContain('commands.connectionState(), commands.agents(), ])')
    expect(source).toContain('show(screen(state, agents))')
    expect(source).not.toMatch(/session/i)
  })

  it('shows the connection bar on home, an agent and a terminal of another machine, and puts the board, the agent and the toasts below it', () => {
    expect(source).toContain(
      "const barred = $derived(route.name === 'home' || route.name === 'agent' || route.name === 'terminal')",
    )
    expect(source).toContain(
      "{:else} {#if barred} <ConnectionBar connection={current.connection} {machineName} bind:height={barHeight} onmachines={() => router.go({ name: 'machines' })} /> {/if}",
    )
    expect(source).toContain(
      'style:transform={translated(shift)}> {#if route.name === \'home\'} <Board',
    )
    expect(source).toContain('{:else} <NoMachines onadded={added} /> {/if}')
    expect(source).toContain(
      "<Board machine={current.machine} agents={current.agents} connection={current.connection} below={barHeight} onopen={open} onstart={start} />",
    )
    expect(source).toContain(
      "const open = (pane: number) => router.go({ name: 'agent', machine: current.machine, pane })",
    )
    expect(source).toContain(
      '<ToastStack items={toasts.items} ondismiss={(id) => toasts.dismiss(id)} below={barred ? barHeight : 0} />',
    )
  })

  it('opens a chat for the tapped pane on its own machine, kept alive under its terminal and under Machines, and goes back from it', () => {
    expect(source).toContain('const opening = $derived(router.opened)')
    expect(source).toContain('const pane = $derived(opening?.pane ?? null)')
    expect(source).toContain('const owner = $derived(opening?.machine ?? null)')
    expect(source).toContain(
      'const chat = $derived( pane === null ? null : new Chat( owner, pane, commands.lastMessage, commands.send, commands.keys, commands.markReviewed, commands.attach, haptics, timers, asker, ), )',
    )
    expect(source).toContain(
      'const asker: Asker = { open: commands.questionOpen, close: commands.questionClose, listen: (heard) => events.questionChanged.listen((event) => heard(event.payload)), }',
    )
    expect(source).toContain(
      "<Agent {chat} agents={current.agents} connection={current.connection} {backTo} below={barHeight} {starting} onback={goBack} onopen={open} onfailed={(message) => toasts.push({ level: 'error', message })} onterminal={terminal} onchanges={review} onfiles={browse} onstart={startIn} />",
    )
  })

  it('gives the chat every list of agents, and reads once its row is seen', () => {
    expect(source).toContain(
      '$effect(() => { const { machine, agents } = current if (chat !== null) { const waiting = starting !== null untrack(() => chat.see(machine, agents, waiting)) } })',
    )
    expect(source).toContain('const seen = $derived(chat?.row != null)')
    expect(source).toContain('$effect(() => { if (chat !== null && seen) { untrack(() => chat.refresh()) } })')
  })

  it('watches the chat for done only while its agent or its terminal is on top and the page shows', () => {
    expect(source).toContain('const inView = $derived(chatOf(route) === null ? null : chat)')
    expect(source).toContain(
      '$effect(() => { const watched = inView if (watched !== null) { return untrack(() => whileShown(document, () => watched.watchDone())) } })',
    )
  })

  it("names the route below for the back button, from the saved machines and the active machine's rows", () => {
    expect(source).toContain(
      'const lookups: Lookups = { machineName: (id) => machines.find((machine) => machine.id === id)?.name, agentRow: (opened) => opened.machine === current.machine ? current.agents?.find((row) => row.pane === opened.pane) : undefined, listed: () => current.agents ?? [], }',
    )
    expect(source).toContain('const backTo = $derived(titleOf(router.below, lookups))')
  })

  it('opens Start from the board on the active machine, with its connection and without the bar', () => {
    expect(source).toContain(
      "const start = () => { if (current.machine !== null) { router.go({ name: 'start', machine: current.machine }) } }",
    )
    expect(source).not.toContain('boot')
    expect(source).toContain(
      "{:else if route.name === 'start'} <Start connection={current.connection} issue={route.issue} workspace={route.workspace} {backTo} onback={goBack} onstarted={started} onissues={listIssues} /> {:else if route.name === 'issues'}",
    )
  })

  it("opens Start from an agent on the active machine, with the agent's workspace chosen", () => {
    expect(source).toContain(
      "const startIn = (workspace: number) => { if (current.machine !== null) { router.go({ name: 'start', machine: current.machine, workspace }) } }",
    )
  })

  it('lists the issues on the machine of Start, opens a tapped one with its project and starts on it where it may, from the workspace Start was opened on', () => {
    expect(source).toContain(
      "const listIssues = () => { const at = router.current if (at.name === 'start') { router.go({ name: 'issues', machine: at.machine }) } }",
    )
    expect(source).toContain(
      "const openIssue = (row: Found) => { if (issues !== null) { const { project, repository, key, source } = row router.go({ name: 'issue', machine: issues.machine, project, repository, key, source }) } }",
    )
    expect(source).toContain(
      "const whereOf = (shown: Extract<Route, { name: 'issue' }>): Where => issues?.whereOf(shown, router.latest('start')?.workspace) ?? { project: null, places: [] }",
    )
    expect(source).toContain(
      "const startOn = (starting: StartingIssue) => { if (issue !== null) { router.go({ name: 'start', machine: issue.machine, issue: starting }) } }",
    )
    expect(source).toContain(
      "{:else if route.name === 'issues'} {#if issues !== null} <Issues {issues} {backTo} onback={goBack} onopen={openIssue} /> {/if} {:else if route.name === 'issue'} {#if issue !== null} <Issue {issue} where={whereOf(route)} {backTo} onback={goBack} onstart={startOn} /> {/if} {:else if route.name === 'machines'}",
    )
  })

  it('keeps one state of the listed project and one of the read issue while they are in the stack, reading them on untracked and leaving them after', () => {
    expect(source).toContain("const listing = $derived(router.latest('issues'))")
    expect(source).toContain(
      'const issues = $derived(listing === null ? null : new IssuesState(listing.machine, commands))',
    )
    expect(source).toContain("const shown = $derived(router.latest('issue'))")
    expect(source).toContain(
      'const issue = $derived(shown === null ? null : new IssueState(shown.machine, shown, commands))',
    )
    expect(source).toContain(
      'const kept = (state: IssuesState | IssueState | null) => { if (state === null) { return } untrack(() => void state.load()) return () => state.stop() }',
    )
    expect(source).toContain('$effect(() => kept(issues)) $effect(() => kept(issue))')
  })

  it('reads the issues again when the connection comes back', () => {
    expect(source).toContain(
      "$effect(() => { const connected = current.connection.state === 'connected' const watched = [issues, issue] untrack(() => watched.forEach((state) => void state?.see(connected))) })",
    )
  })

  it('puts the agent it started on the board in place of Start and all under it, says it is starting and tells its warning', () => {
    expect(source).toContain(
      "const started = (placed: Started, agent: string) => { const at = router.current if (at.name === 'start') { launched = agent router.settle({ name: 'agent', machine: at.machine, pane: placed.pane, starting: true }) } else { toasts.push({ level: 'info', message: `${agent} started` }) } if (placed.warning !== null) { toasts.push({ level: 'info', message: placed.warning }) } }",
    )
    expect(source).toContain(
      "const starting = $derived(route.name === 'agent' && route.starting === true ? launched : null)",
    )
  })

  it('reads again on a status change of its pane on its machine', () => {
    expect(source).toContain(
      'events.paneStatusChanged.listen((event) => { if (chat !== null && touches(event.payload, chat.machine, chat.pane)) { void chat.refresh() } }),',
    )
  })

  it("opens an agent's terminal on top of it, on the machine the agent runs on", () => {
    expect(source).toContain(
      "const terminal = (row: AgentRow) => { if (current.machine !== null) { router.go({ name: 'terminal', machine: current.machine, row }) } }",
    )
  })

  it('shows the terminal on top of everything only on its own machine, going back from it and putting the toasts below its bar', () => {
    expect(source).toContain('const viewed = $derived(openedTerminal(route, current))')
    expect(source).toContain(
      '{#if viewed !== undefined} <Terminal machine={viewed.machine} row={viewed.row} {backTo} {shift} onback={goBack} onfailed={toasts.failed} bind:height={barHeight} />',
    )
  })

  it("opens the changes of an agent's workspace on top of it, on the active machine, and their diff above them", () => {
    expect(source).toContain(
      "const review = (workspace: number, title: string) => { if (current.machine !== null) { router.go({ name: 'changes', machine: current.machine, workspace, title }) } }",
    )
    expect(source).toContain(
      "{:else if route.name === 'changes'} {#if changes !== null} <Changes title={route.title} {changes} {backTo} onback={goBack} onopen={change} /> {/if} {:else if route.name === 'change'} {#if changes !== null} <Diff path={route.path} {changes} {backTo} onback={goBack} onfile={whole} /> {/if} {:else if route.name === 'files'}",
    )
  })

  it("reads a file of the list and opens its diff in the list's mode", () => {
    expect(source).toContain(
      "const change = (path: string) => { if (changes !== null) { void changes.open(path) router.go({ name: 'change', machine: changes.machine, workspace: changes.workspace, path }) } }",
    )
  })

  it('goes back through one action that drops the open diff only when leaving it', () => {
    expect(source).toContain(
      "const goBack = () => { const top = router.current.name if (top === 'change') { changes?.closeDiff() }",
    )
    expect(source.match(/onback=\{goBack\}/g)).toHaveLength(11)
    expect(source).not.toContain('onback={() =>')
    expect(source).not.toContain('leaveDiff')
  })

  it('keeps one state of the reviewed workspace under its list and its diff, following it once it listens and leaving it after', () => {
    expect(source).toContain('const reviewedMachine = $derived(reviewed?.machine ?? null)')
    expect(source).toContain('const reviewedWorkspace = $derived(reviewed?.workspace ?? null)')
    expect(source).toContain(
      'follow: commands.changesFollow, unfollow: commands.changesUnfollow, read: commands.changesFile, mark: commands.changesViewed, }, (toast) => toasts.push(toast),',
    )
    expect(source).toContain(
      'const listening = events.changesChanged.listen((event) => state.see(event.payload)) listening.then(() => state.start()).catch(toasts.failed) return () => { state.stop() listening.then((unlisten) => unlisten()).catch(toasts.failed) }',
    )
  })

  it("opens the files of an agent's workspace on top of it, on the active machine, and a file above them", () => {
    expect(source).toContain(
      "const browse = (workspace: number, title: string) => { if (current.machine !== null) { router.go({ name: 'files', machine: current.machine, workspace, title }) } }",
    )
    expect(source).toContain(
      "{:else if route.name === 'files'} {#if files !== null} <Files title={route.title} {files} {backTo} onback={goBack} onopen={read} /> {/if} {:else if route.name === 'file'} {#if files !== null} <File path={route.path} {files} {backTo} onback={goBack} /> {/if} {:else if route.name === 'start'}",
    )
  })

  it('reads a file of the tree each time it opens it', () => {
    expect(source).toContain(
      "const read = (path: string) => { if (files !== null) { void files.open(path) router.go({ name: 'file', machine: files.machine, workspace: files.workspace, path }) } }",
    )
  })

  it('drops the open file before going back from it, then lists the tree again', () => {
    expect(source).toContain(
      "if (top === 'file') { files?.closeFile() } router.back() if (top === 'file') { void files?.refresh() } }",
    )
    expect(source).not.toContain('leaveFile')
  })

  it('opens the whole file of a diff above it, on the same workspace, keeping the diff and its state under it', () => {
    expect(source).toContain(
      "const whole = (path: string) => { if (changes !== null) { router.go({ name: 'file', machine: changes.machine, workspace: changes.workspace, path }) } }",
    )
  })

  it('keeps one state of the browsed workspace under its tree and its file, listing the tree or reading the file it opened on untracked, so its own writes do not stop it, and leaving it after', () => {
    expect(source).toContain('const browsedMachine = $derived(browsed?.machine ?? null)')
    expect(source).toContain('const browsedWorkspace = $derived(browsed?.workspace ?? null)')
    expect(source).toContain(
      '{ list: commands.filesList, read: commands.filesRead, search: commands.filesSearch, stopSearch: commands.filesSearchStop, }, (toast) => toasts.push(toast),',
    )
    expect(source).toContain(
      "untrack(() => { const top = router.current void (top.name === 'file' ? state.open(top.path) : state.start()) }) return () => state.stop()",
    )
  })

  it('goes back with the edge swipe only on iOS over another screen, and never while the scanner is open', () => {
    expect(source).toContain('const swipeable = $derived(backSwipes(platform, router.below))')
    expect(source).toContain(
      "$effect(() => { if (!swipeable) { return } return watchBackSwipe(document, { enabled: () => !document.documentElement.classList.contains('scanning'), width: () => innerWidth, reduced: prefersReducedMotion, onshift: (next) => (shift = next), onback: goBack, }) })",
    )
    expect(source.match(/watchBackSwipe\(/g)).toHaveLength(1)
  })

  it("takes android's back button only while it goes back or cancels the scanner, deciding when pressed", () => {
    expect(source).toContain('const pressed = () => backPressed(platform, router.below, openScanner.scanning)')
    expect(source).toContain('const backHandled = $derived(handled(pressed()))')
    expect(source).toContain(
      '$effect(() => { if (!backHandled) { return } return watchBackButton({ pressed, onback: goBack, onscan: () => void openScanner.stop(), failed: toasts.failed }) })',
    )
    expect(source.match(/watchBackButton\(/g)).toHaveLength(1)
  })

  it('slides every screen but the connection bar and the toasts, and the terminal on its own', () => {
    expect(source).toContain('let shift = $state<Shift>(RESTING)')
    expect(source).toContain(
      "onmachines={() => router.go({ name: 'machines' })} /> {/if} <div class=\"page\" class:sliding={shift.x > 0} class:settling={shift.eased} style:transform={translated(shift)}>",
    )
    expect(source).toContain('<NoMachines onadded={added} /> {/if} </div> {/if} <ToastStack')
    expect(source.match(/translated\(shift\)/g)).toHaveLength(1)
  })

  it('shows a terminal of another machine as the gone agent', () => {
    expect(source).toContain(
      "{:else if barred} {#if chat !== null} <Agent {chat} agents={current.agents} connection={current.connection} {backTo} below={barHeight} {starting} onback={goBack} onopen={open} onfailed={(message) => toasts.push({ level: 'error', message })} onterminal={terminal} onchanges={review} onfiles={browse} onstart={startIn} /> {/if} {:else if route.name === 'changes'}",
    )
  })
})
