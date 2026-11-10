<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core'
  import * as haptics from '@tauri-apps/plugin-haptics'
  import { onMount, untrack } from 'svelte'
  import {
    commands,
    events,
    type AgentRow_Serialize as AgentRow,
    type MachineView_Serialize,
    type Started_Serialize as Started,
  } from './lib/bindings'
  import type { Images } from './lib/attach.svelte'
  import { backPressed, handled, watchBackButton } from './lib/backButton'
  import { backSwipes, RESTING, translated, watchBackSwipe, type Shift } from './lib/backSwipe'
  import { ChangesState } from './lib/changes.svelte'
  import { Chat, timers, touches, whileShown } from './lib/chat.svelte'
  import { apply, connecting, firstToConnect, screen, type Screen } from './lib/connection'
  import { FilesState } from './lib/files.svelte'
  import { IssuesState, IssueState } from './lib/issues.svelte'
  import type { Found } from './lib/issueSweep'
  import { latestWins } from './lib/join'
  import { titleOf, type Lookups } from './lib/navigation'
  import { platform } from './lib/platform'
  import { chatOf, openedTerminal, Router, routeFor, type Route, type StartingIssue, type Where } from './lib/router.svelte'
  import type { Asker } from './lib/question.svelte'
  import { openScanner } from './lib/scanner.svelte'
  import { Toasts } from './lib/toasts.svelte'
  import ConnectionBar from './lib/ui/ConnectionBar.svelte'
  import { prefersReducedMotion } from './lib/ui/motion'
  import ToastStack from './lib/ui/Toasts.svelte'
  import Agent from './routes/agent/Agent.svelte'
  import Board from './routes/board/Board.svelte'
  import Changes from './routes/changes/Changes.svelte'
  import Diff from './routes/changes/Diff.svelte'
  import File from './routes/files/File.svelte'
  import Files from './routes/files/Files.svelte'
  import Issue from './routes/issues/Issue.svelte'
  import Issues from './routes/issues/Issues.svelte'
  import Machine from './routes/Machine.svelte'
  import Machines from './routes/Machines.svelte'
  import NoMachines from './routes/NoMachines.svelte'
  import Start from './routes/start/Start.svelte'
  import Terminal from './routes/terminal/Terminal.svelte'

  type Listed = Awaited<ReturnType<typeof commands.machinesList>>

  const idle = screen({ machine: null, connection: { state: 'none' } }, null)

  let current = $state(idle)
  let machines = $state<MachineView_Serialize[]>([])
  let barHeight = $state(0)
  let launched = $state('')
  let shift = $state<Shift>(RESTING)
  const router = new Router(routeFor(idle, 0))
  const toasts = new Toasts()
  const images: Images = { attach: commands.attach, paste: commands.pasteAttached, discard: commands.discard }
  const asker: Asker = {
    open: commands.questionOpen,
    close: commands.questionClose,
    listen: (heard) => events.questionChanged.listen((event) => heard(event.payload)),
  }

  const route = $derived(router.current)
  const barred = $derived(route.name === 'home' || route.name === 'agent' || route.name === 'terminal')
  const opening = $derived(router.opened)
  const pane = $derived(opening?.pane ?? null)
  const owner = $derived(opening?.machine ?? null)
  const chat = $derived(
    pane === null
      ? null
      : new Chat(
          owner,
          pane,
          commands.lastMessage,
          commands.send,
          commands.keys,
          commands.markReviewed,
          images,
          haptics,
          timers,
          asker,
        ),
  )
  const seen = $derived(chat?.row != null)
  const inView = $derived(chatOf(route) === null ? null : chat)
  const viewed = $derived(openedTerminal(route, current))
  const editing = $derived(route.name === 'machine' ? machines.find((machine) => machine.id === route.id) : undefined)
  const rescanning = $derived(route.name === 'machine' && route.adding === true)
  const taken = $derived(machines.filter((machine) => machine.id !== editing?.id).map((machine) => machine.name))
  const machineName = $derived(machines.find((machine) => machine.id === current.machine)?.name ?? '')
  const starting = $derived(route.name === 'agent' && route.starting === true ? launched : null)
  const reviewed = $derived(router.reviewed)
  const reviewedMachine = $derived(reviewed?.machine ?? null)
  const reviewedWorkspace = $derived(reviewed?.workspace ?? null)
  const lookups: Lookups = {
    machineName: (id) => machines.find((machine) => machine.id === id)?.name,
    agentRow: (opened) =>
      opened.machine === current.machine ? current.agents?.find((row) => row.pane === opened.pane) : undefined,
    listed: () => current.agents ?? [],
  }
  const backTo = $derived(titleOf(router.below, lookups))
  const swipeable = $derived(backSwipes(platform, router.below))
  const pressed = () => backPressed(platform, router.below, openScanner.scanning)
  const backHandled = $derived(handled(pressed()))
  const changes = $derived(
    reviewedMachine === null || reviewedWorkspace === null
      ? null
      : new ChangesState(
          reviewedMachine,
          reviewedWorkspace,
          {
            follow: commands.changesFollow,
            unfollow: commands.changesUnfollow,
            read: commands.changesFile,
            mark: commands.changesViewed,
          },
          (toast) => toasts.push(toast),
        ),
  )

  const browsed = $derived(router.browsed)
  const browsedMachine = $derived(browsed?.machine ?? null)
  const browsedWorkspace = $derived(browsed?.workspace ?? null)
  const files = $derived(
    browsedMachine === null || browsedWorkspace === null
      ? null
      : new FilesState(
          browsedMachine,
          browsedWorkspace,
          {
            list: commands.filesList,
            read: commands.filesRead,
            search: commands.filesSearch,
            stopSearch: commands.filesSearchStop,
          },
          (toast) => toasts.push(toast),
        ),
  )

  const listing = $derived(router.latest('issues'))
  const issues = $derived(listing === null ? null : new IssuesState(listing.machine, commands))
  const shown = $derived(router.latest('issue'))
  const issue = $derived(shown === null ? null : new IssueState(shown.machine, shown, commands))

  $effect(() => {
    const { machine, agents } = current
    if (chat !== null) {
      const waiting = starting !== null
      untrack(() => chat.see(machine, agents, waiting))
    }
  })

  $effect(() => {
    if (chat !== null && seen) {
      untrack(() => chat.refresh())
    }
  })

  $effect(() => {
    const opened = chat
    return () => opened?.leave()
  })

  $effect(() => {
    const watched = inView
    if (watched !== null) {
      return untrack(() => whileShown(document, () => watched.watchDone()))
    }
  })

  $effect(() => {
    const state = changes
    if (state === null) {
      return
    }
    const listening = events.changesChanged.listen((event) => state.see(event.payload))
    listening.then(() => state.start()).catch(toasts.failed)
    return () => {
      state.stop()
      listening.then((unlisten) => unlisten()).catch(toasts.failed)
    }
  })

  $effect(() => {
    const state = files
    if (state === null) {
      return
    }
    untrack(() => {
      const top = router.current
      void (top.name === 'file' ? state.open(top.path) : state.start())
    })
    return () => state.stop()
  })

  const kept = (state: IssuesState | IssueState | null) => {
    if (state === null) {
      return
    }
    untrack(() => void state.load())
    return () => state.stop()
  }

  $effect(() => kept(issues))

  $effect(() => kept(issue))

  $effect(() => {
    const connected = current.connection.state === 'connected'
    const watched = [issues, issue]
    untrack(() => watched.forEach((state) => void state?.see(connected)))
  })

  const show = (next: Screen) => {
    current = next
    router.setBase(routeFor(next, machines.length))
  }

  const open = (pane: number) => router.go({ name: 'agent', machine: current.machine, pane })

  const terminal = (row: AgentRow) => {
    if (current.machine !== null) {
      router.go({ name: 'terminal', machine: current.machine, row })
    }
  }

  const review = (workspace: number, title: string) => {
    if (current.machine !== null) {
      router.go({ name: 'changes', machine: current.machine, workspace, title })
    }
  }

  const change = (path: string) => {
    if (changes !== null) {
      void changes.open(path)
      router.go({ name: 'change', machine: changes.machine, workspace: changes.workspace, path })
    }
  }

  const browse = (workspace: number, title: string) => {
    if (current.machine !== null) {
      router.go({ name: 'files', machine: current.machine, workspace, title })
    }
  }

  const read = (path: string) => {
    if (files !== null) {
      void files.open(path)
      router.go({ name: 'file', machine: files.machine, workspace: files.workspace, path })
    }
  }

  const whole = (path: string) => {
    if (changes !== null) {
      router.go({ name: 'file', machine: changes.machine, workspace: changes.workspace, path })
    }
  }

  const goBack = () => {
    const top = router.current.name
    if (top === 'change') {
      changes?.closeDiff()
    }
    if (top === 'file') {
      files?.closeFile()
    }
    router.back()
    if (top === 'file') {
      void files?.refresh()
    }
  }

  $effect(() => {
    if (!swipeable) {
      return
    }
    return watchBackSwipe(document, {
      enabled: () => !document.documentElement.classList.contains('scanning'),
      width: () => innerWidth,
      reduced: prefersReducedMotion,
      onshift: (next) => (shift = next),
      onback: goBack,
    })
  })

  $effect(() => {
    if (!backHandled) {
      return
    }
    return watchBackButton({ pressed, onback: goBack, onscan: () => void openScanner.stop(), failed: toasts.failed })
  })

  const start = () => {
    if (current.machine !== null) {
      router.go({ name: 'start', machine: current.machine })
    }
  }

  const startIn = (workspace: number) => {
    if (current.machine !== null) {
      router.go({ name: 'start', machine: current.machine, workspace })
    }
  }

  const started = (placed: Started, agent: string) => {
    const at = router.current
    if (at.name === 'start') {
      launched = agent
      router.settle({ name: 'agent', machine: at.machine, pane: placed.pane, starting: true })
    } else {
      toasts.push({ level: 'info', message: `${agent} started` })
    }
    if (placed.warning !== null) {
      toasts.push({ level: 'info', message: placed.warning })
    }
  }

  const listIssues = () => {
    const at = router.current
    if (at.name === 'start') {
      router.go({ name: 'issues', machine: at.machine })
    }
  }

  const openIssue = (row: Found) => {
    if (issues !== null) {
      const { project, repository, key, source } = row
      router.go({ name: 'issue', machine: issues.machine, project, repository, key, source })
    }
  }

  const whereOf = (shown: Extract<Route, { name: 'issue' }>): Where =>
    issues?.whereOf(shown, router.latest('start')?.workspace) ?? { project: null, places: [] }

  const startOn = (starting: StartingIssue) => {
    if (issue !== null) {
      router.go({ name: 'start', machine: issue.machine, issue: starting })
    }
  }

  const remember = (listed: Listed) => {
    if (listed.status === 'error') {
      toasts.failed(listed.error)
      return { machines: [], active: null }
    }
    machines = listed.data.machines
    return listed.data
  }

  const reload = () => commands.machinesList().then(remember).catch(toasts.failed)

  const removed = async () => {
    router.back()
    await reload()
    router.setBase(routeFor(current, machines.length))
  }

  const join = latestWins(async (machine: number) => {
    show(connecting(machine))
    const connected = await commands.connect(machine)
    if (connected.status === 'error') {
      toasts.failed(connected.error)
    }
  })

  const added = (machine: number) => {
    show(connecting(machine))
    Promise.all([commands.machinesList().then(remember), join(machine)]).catch(toasts.failed)
  }

  const load = async () => {
    const [listed, state, agents] = await Promise.all([
      commands.machinesList(),
      commands.connectionState(),
      commands.agents(),
    ])
    const known = remember(listed)
    show(screen(state, agents))
    const first = firstToConnect(known, state)
    if (first !== null) {
      await join(first)
    }
  }

  onMount(() => {
    if (!isTauri()) {
      return
    }
    const listening = [
      events.connectionChanged.listen((event) => show(apply(current, event.payload))),
      events.agentsChanged.listen((event) => show(apply(current, event.payload))),
      events.paneStatusChanged.listen((event) => {
        if (chat !== null && touches(event.payload, chat.machine, chat.pane)) {
          void chat.refresh()
        }
      }),
      events.toast.listen((event) => toasts.push(event.payload)),
    ]
    Promise.all(listening).then(load).catch(toasts.failed)
    return () => listening.forEach((stop) => stop.then((unlisten) => unlisten()))
  })
</script>

<main>
  {#if viewed !== undefined}
    <Terminal
      machine={viewed.machine}
      row={viewed.row}
      {backTo}
      {shift}
      onback={goBack}
      onfailed={toasts.failed}
      bind:height={barHeight}
    />
  {:else}
    {#if barred}
      <ConnectionBar
        connection={current.connection}
        {machineName}
        bind:height={barHeight}
        onmachines={() => router.go({ name: 'machines' })}
      />
    {/if}
    <div class="page" class:sliding={shift.x > 0} class:settling={shift.eased} style:transform={translated(shift)}>
      {#if route.name === 'home'}
        <Board
          machine={current.machine}
          agents={current.agents}
          connection={current.connection}
          below={barHeight}
          onopen={open}
          onstart={start}
        />
      {:else if barred}
        {#if chat !== null}
          <Agent
            {chat}
            agents={current.agents}
            connection={current.connection}
            {backTo}
            below={barHeight}
            {starting}
            onback={goBack}
            onopen={open}
            onfailed={(message) => toasts.push({ level: 'error', message })}
            ontoast={(toast) => toasts.push(toast)}
            onterminal={terminal}
            onchanges={review}
            onfiles={browse}
            onstart={startIn}
          />
        {/if}
      {:else if route.name === 'changes'}
        {#if changes !== null}
          <Changes title={route.title} {changes} {backTo} onback={goBack} onopen={change} />
        {/if}
      {:else if route.name === 'change'}
        {#if changes !== null}
          <Diff path={route.path} {changes} {backTo} onback={goBack} onfile={whole} />
        {/if}
      {:else if route.name === 'files'}
        {#if files !== null}
          <Files title={route.title} {files} {backTo} onback={goBack} onopen={read} />
        {/if}
      {:else if route.name === 'file'}
        {#if files !== null}
          <File path={route.path} {files} {backTo} onback={goBack} />
        {/if}
      {:else if route.name === 'start'}
        <Start
          connection={current.connection}
          issue={route.issue}
          workspace={route.workspace}
          {backTo}
          onback={goBack}
          onstarted={started}
          onissues={listIssues}
        />
      {:else if route.name === 'issues'}
        {#if issues !== null}
          <Issues {issues} {backTo} onback={goBack} onopen={openIssue} />
        {/if}
      {:else if route.name === 'issue'}
        {#if issue !== null}
          <Issue {issue} where={whereOf(route)} {backTo} onback={goBack} onstart={startOn} />
        {/if}
      {:else if route.name === 'machines'}
        <Machines
          {machines}
          active={current.machine}
          connection={current.connection}
          {backTo}
          onback={goBack}
          onpick={(machine) => void join(machine)}
          onadded={added}
          onrouted={reload}
          onedit={(machine) => router.go({ name: 'machine', id: machine })}
          onrescan={(machine) => router.go({ name: 'machine', id: machine, adding: true })}
        />
      {:else if editing !== undefined}
        <Machine
          machine={editing}
          {taken}
          connection={editing.id === current.machine ? current.connection : null}
          {backTo}
          onback={goBack}
          onchanged={reload}
          onremoved={removed}
          adding={rescanning}
        />
      {:else}
        <NoMachines onadded={added} />
      {/if}
    </div>
  {/if}
  <ToastStack items={toasts.items} ondismiss={(id) => toasts.dismiss(id)} below={barred ? barHeight : 0} />
</main>
