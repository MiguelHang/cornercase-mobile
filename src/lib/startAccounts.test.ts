import { describe, expect, it, vi } from 'vitest'
import type {
  AccountChoice_Serialize as AccountChoice,
  AgentAccounts_Serialize as AgentAccounts,
  CommandError,
  Login_Serialize as Login,
} from './bindings'
import { StartAccounts } from './startAccounts.svelte'
import { StartForm, type StartCalls } from './startForm.svelte'

const account = (name: string, label: string, login: Login = 'signed-in'): AccountChoice => ({ name, label, login })

const CLAUDE: AgentAccounts = {
  agent: 'claude',
  default: '',
  options: [account('', 'Mac login'), account('work', 'Work'), account('personal', 'personal')],
}

const CODEX: AgentAccounts = {
  agent: 'codex',
  default: 'team',
  options: [
    account('', 'System', 'unknown'),
    account('team', 'Team'),
    account('old', 'Old', 'signed-out'),
    account('broken', 'Broken', 'failed'),
  ],
}

const reading = (...known: AgentAccounts[]) => {
  const accounts = new StartAccounts()
  accounts.read(known)
  return accounts
}

describe('StartAccounts', () => {
  it('offers the own login first and then each account, in the order cornercase gives', () => {
    expect(reading(CLAUDE, CODEX).optionsOf('claude')).toEqual([
      { value: '', name: 'Mac login' },
      { value: 'work', name: 'Work' },
      { value: 'personal', name: 'personal' },
    ])
  })

  it('marks the accounts signed out or that could not be checked, and leaves the rest plain', () => {
    expect(reading(CODEX).optionsOf('codex').map((option) => option.name)).toEqual([
      'System',
      'Team',
      'Old · signed out',
      'Broken · could not check',
    ])
  })

  it.each([
    ['the active account', 'codex', 'team'],
    ['the own login when no account is active', 'claude', ''],
  ])('preselects %s', (_, agent, chosen) => {
    expect(reading(CLAUDE, CODEX).chosen(agent)).toBe(chosen)
  })

  it('sends no account while each agent keeps its default, also when it is chosen again', () => {
    const accounts = reading(CLAUDE, CODEX)
    accounts.choose('claude', 'work')
    accounts.choose('claude', '')
    accounts.choose('codex', 'team')
    expect(accounts.picks(['claude', 'codex'])).toEqual([])
  })

  it('sends another account, and the own login when the default is an added account, in the order given', () => {
    const accounts = reading(CLAUDE, CODEX)
    accounts.choose('claude', 'work')
    accounts.choose('codex', '')
    expect(accounts.picks(['codex', 'claude'])).toEqual([
      { agent: 'codex', account: '' },
      { agent: 'claude', account: 'work' },
    ])
    expect(accounts.picks(['claude'])).toEqual([{ agent: 'claude', account: 'work' }])
  })

  it('keeps the choice of each agent when the form goes to another agent and back', () => {
    const accounts = reading(CLAUDE, CODEX)
    accounts.choose('claude', 'personal')
    expect(accounts.chosen('codex')).toBe('team')
    accounts.choose('codex', 'old')
    expect([accounts.chosen('claude'), accounts.chosen('codex')]).toEqual(['personal', 'old'])
  })

  it('offers and sends nothing for an agent without accounts', () => {
    const accounts = reading(CLAUDE)
    accounts.choose('gemini', 'work')
    expect(accounts.optionsOf('gemini')).toEqual([])
    expect(accounts.chosen('gemini')).toBe('')
    expect(accounts.signedOut('gemini')).toBe(false)
    expect(accounts.picks(['gemini', 'claude'])).toEqual([])
  })

  it('says when the chosen account is signed out', () => {
    const accounts = reading(CODEX)
    expect(accounts.signedOut('codex')).toBe(false)
    accounts.choose('codex', 'old')
    expect(accounts.signedOut('codex')).toBe(true)
    accounts.choose('codex', 'broken')
    expect(accounts.signedOut('codex')).toBe(false)
  })

  it('goes back to the default when the chosen account is no longer listed', () => {
    const accounts = reading(CLAUDE)
    accounts.choose('claude', 'personal')
    accounts.read([{ ...CLAUDE, options: CLAUDE.options.slice(0, 2) }])
    expect(accounts.chosen('claude')).toBe('')
    expect(accounts.picks(['claude'])).toEqual([])
  })
})

const ok = <T>(data: T) => Promise.resolve({ status: 'ok' as const, data })

const OFFLINE: CommandError = { code: 'not_connected', message: 'offline' }

const WITH_ACCOUNTS = { agents: [{ agent: 'claude', command: 'claude', installed: true }], accounts: [CLAUDE] }

const formWith = (installedAgents: StartCalls['installedAgents'], boots: string[] = ['boot-1']) => {
  const calls = {
    startProjects: () => ok({ boot: boots.shift() ?? 'boot-1', projects: [] }),
    installedAgents: vi.fn(installedAgents),
    startAgent: vi.fn(),
    startIssue: vi.fn(),
    issueFlows: vi.fn(),
    startFlow: vi.fn(),
  }
  return new StartForm(calls, vi.fn(), { agent: null, workspace: null })
}

describe('StartForm accounts', () => {
  it('reads the accounts with the agents, and offers none while they are read', async () => {
    let open: () => void = () => {}
    const gate = new Promise<void>((resolve) => (open = resolve))
    const form = formWith(async () => {
      await gate
      return ok(WITH_ACCOUNTS)
    })
    const loading = form.load()
    expect(form.accounts.optionsOf('claude')).toEqual([])
    open()
    await loading
    expect(form.accounts.optionsOf('claude')).toHaveLength(3)
  })

  it('offers none when the agents failed, and reads them again with Try again', async () => {
    const answers = [Promise.resolve({ status: 'error' as const, error: OFFLINE }), ok(WITH_ACCOUNTS)]
    const form = formWith(() => answers.shift() ?? ok(WITH_ACCOUNTS))
    await form.load()
    expect(form.accounts.optionsOf('claude')).toEqual([])
    await form.loadAgents()
    expect(form.accounts.optionsOf('claude')).toHaveLength(3)
  })

  it('keeps the choices on the same server, and goes back to the defaults after it restarted', async () => {
    const form = formWith(() => ok(WITH_ACCOUNTS), ['boot-1', 'boot-1', 'boot-2'])
    await form.load()
    form.accounts.choose('claude', 'work')
    await form.see(false)
    await form.see(true)
    expect(form.accounts.chosen('claude')).toBe('work')
    await form.see(false)
    await form.see(true)
    expect(form.accounts.chosen('claude')).toBe('')
    expect(form.accounts.optionsOf('claude')).toHaveLength(3)
  })
})
