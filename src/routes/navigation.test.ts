import { describe, expect, it } from 'vitest'

const screens = import.meta.glob<string>('/src/routes/**/*.svelte', {
  query: '?raw',
  import: 'default',
  eager: true,
})

const OLD = ['class="top"', 'class="back"', '>Back<', 'Back to the board']

const leftovers = (text: string) => OLD.filter((piece) => text.includes(piece))

const withBar = Object.entries(screens)
  .filter(([, text]) => text.includes('<NavBar'))
  .map(([path]) => path)

describe('the screens of src/routes', () => {
  it('reads every screen', () => {
    expect(Object.keys(screens)).toContain('/src/routes/terminal/Terminal.svelte')
    expect(Object.keys(screens).length).toBeGreaterThan(20)
  })

  it.each(Object.entries(screens))('%s keeps no old header', (_, text) => {
    expect(leftovers(text)).toEqual([])
  })

  it('go back through the NavBar on every screen with a way back', () => {
    expect(withBar.sort()).toEqual([
      '/src/routes/Machine.svelte',
      '/src/routes/Machines.svelte',
      '/src/routes/agent/Agent.svelte',
      '/src/routes/agent/ChatHeader.svelte',
      '/src/routes/changes/Changes.svelte',
      '/src/routes/changes/Diff.svelte',
      '/src/routes/start/Start.svelte',
      '/src/routes/terminal/Terminal.svelte',
    ])
  })
})

describe('the guard', () => {
  it.each([
    ['a top bar', '<header class="top">', ['class="top"']],
    ['a back button', '<button class="back" onclick={onback}>', ['class="back"']],
    ['a back label', '<button>Back</button>', ['>Back<']],
    ['the board label', '<button aria-label="Back to the board">', ['Back to the board']],
    ['the NavBar', '<NavBar backTo="Studio" {onback} />', []],
  ])('finds %s', (_, text, found) => {
    expect(leftovers(text)).toEqual(found)
  })
})
